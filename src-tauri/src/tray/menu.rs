use super::scope;
use super::types::{QUICK_SWITCH_MAX_PER_PLATFORM, QUICK_SWITCH_MENU_PREFIX};
use crate::db::Database;
use crate::models::{AccountStatus, IdeAccount};
use crate::services::event_bus::EventBus;
use crate::services::ide_injector::IdeInjector;
use crate::services::provider::ProviderService;
use crate::services::provider_current::ProviderCurrentService;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_notification::NotificationExt;

fn is_platform_in_scope(scope: &[String], platform: &str) -> bool {
    scope.is_empty() || scope.iter().any(|item| item.eq_ignore_ascii_case(platform))
}

fn provider_platform_key(platform: &crate::models::Platform) -> String {
    serde_json::to_string(platform)
        .unwrap_or_else(|_| "\"unknown\"".to_string())
        .trim_matches('"')
        .to_ascii_lowercase()
}

fn account_platform_key(platform: &str) -> String {
    platform.trim().to_ascii_lowercase()
}

fn is_attention_account(account: &IdeAccount) -> bool {
    account.is_proxy_disabled
        || account
            .disabled_reason
            .as_ref()
            .is_some_and(|reason| !reason.trim().is_empty())
        || !matches!(account.status, AccountStatus::Active)
}

fn account_status_label(status: &AccountStatus) -> &'static str {
    match status {
        AccountStatus::Active => "正常",
        AccountStatus::Expired => "已过期",
        AccountStatus::Forbidden => "已封禁",
        AccountStatus::RateLimited => "限流中",
        AccountStatus::Unknown => "未知",
    }
}

fn account_display_label(account: &IdeAccount) -> String {
    let base = account
        .label
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(account.email.as_str());
    let mut label = format!("{} · {}", base, account_status_label(&account.status));
    if account.is_proxy_disabled {
        label.push_str(" · 已禁用代理");
    }
    if account
        .disabled_reason
        .as_ref()
        .is_some_and(|reason| !reason.trim().is_empty())
    {
        label.push_str(" · 需关注");
    }
    label
}

pub(super) fn quick_switch_menu_id(platform: &str, account_id: &str) -> String {
    format!(
        "{}{}::{}",
        QUICK_SWITCH_MENU_PREFIX,
        account_platform_key(platform),
        account_id.trim()
    )
}

pub(super) fn parse_quick_switch_menu_id(id: &str) -> Option<(String, String)> {
    let payload = id.strip_prefix(QUICK_SWITCH_MENU_PREFIX)?;
    let (platform, account_id) = payload.split_once("::")?;
    let platform = account_platform_key(platform);
    let account_id = account_id.trim().to_string();
    if platform.is_empty() || account_id.is_empty() {
        return None;
    }
    Some((platform, account_id))
}

pub(super) fn format_accounts_summary_label(
    total: usize,
    current: usize,
    attention: usize,
) -> String {
    format!(
        "账号概览：总 {} · 当前 {} · 需关注 {}",
        total, current, attention
    )
}

pub(super) fn build_menu(app: &AppHandle) -> Result<Menu<Wry>, tauri::Error> {
    let mut menu_items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::new();
    let tray_scope = scope::get_scope_platforms(app);

    let open_item = MenuItem::with_id(app, "open_main", "打开主界面", true, None::<&str>)?;
    menu_items.push(Box::new(open_item));
    let open_accounts_item =
        MenuItem::with_id(app, "open_accounts", "打开账号与资产库", true, None::<&str>)?;
    menu_items.push(Box::new(open_accounts_item));

    let scope_label = if tray_scope.is_empty() {
        "托盘范围：全部平台".to_string()
    } else {
        format!("托盘范围：{}", tray_scope.join(", "))
    };
    let scope_item = MenuItem::with_id(app, "tray_scope_info", scope_label, false, None::<&str>)?;
    menu_items.push(Box::new(scope_item));

    let sep1 = PredefinedMenuItem::separator(app)?;
    menu_items.push(Box::new(sep1));

    let db = app.state::<Database>();
    let providers = ProviderService::new(&*db)
        .list_providers()
        .unwrap_or_default();

    let visible_providers = providers
        .into_iter()
        .filter(|provider| {
            is_platform_in_scope(&tray_scope, &provider_platform_key(&provider.platform))
        })
        .collect::<Vec<_>>();

    if visible_providers.is_empty() {
        let empty_item = MenuItem::with_id(
            app,
            "no_provider",
            "(当前托盘范围内暂无 Provider 配置)",
            false,
            None::<&str>,
        )?;
        menu_items.push(Box::new(empty_item));
    } else {
        for p in visible_providers {
            let id = format!("switch_provider_{}", p.id);
            let prefix = match p.platform {
                crate::models::Platform::Anthropic => "🟠",
                crate::models::Platform::OpenAI => "🟢",
                crate::models::Platform::Gemini => "🔵",
                crate::models::Platform::DeepSeek => "🔷",
                _ => "⚙️",
            };
            let label = format!("{} {}", prefix, p.name);

            let item = CheckMenuItem::with_id(app, &id, &label, true, p.is_active, None::<&str>)?;
            menu_items.push(Box::new(item));
        }
    }

    let snapshots = ProviderCurrentService::list_current_account_snapshots(&*db)
        .unwrap_or_default()
        .into_iter()
        .filter(|item| is_platform_in_scope(&tray_scope, &item.platform))
        .collect::<Vec<_>>();
    let mut visible_accounts = db
        .get_all_ide_accounts()
        .unwrap_or_default()
        .into_iter()
        .filter(|account| {
            is_platform_in_scope(&tray_scope, &account_platform_key(&account.origin_platform))
        })
        .collect::<Vec<_>>();

    let current_count = snapshots
        .iter()
        .filter(|item| {
            item.account_id
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty())
        })
        .count();
    let attention_count = visible_accounts
        .iter()
        .filter(|account| is_attention_account(account))
        .count();

    if !snapshots.is_empty() {
        let sep_snapshot = PredefinedMenuItem::separator(app)?;
        menu_items.push(Box::new(sep_snapshot));

        let snapshot_title = MenuItem::with_id(
            app,
            "current_accounts_title",
            format_accounts_summary_label(visible_accounts.len(), current_count, attention_count),
            false,
            None::<&str>,
        )?;
        menu_items.push(Box::new(snapshot_title));

        for snapshot in snapshots
            .into_iter()
            .filter(|item| item.account_id.is_some())
        {
            let label = format!(
                "{}: {}",
                snapshot.platform,
                snapshot
                    .label
                    .or(snapshot.email)
                    .unwrap_or_else(|| "未解析到当前账号".to_string())
            );
            let item = MenuItem::with_id(
                app,
                format!("current_account_{}", snapshot.platform),
                &label,
                false,
                None::<&str>,
            )?;
            menu_items.push(Box::new(item));
        }
    }

    visible_accounts.sort_by(|a, b| {
        let ap = account_platform_key(&a.origin_platform);
        let bp = account_platform_key(&b.origin_platform);
        ap.cmp(&bp)
            .then_with(|| a.email.to_lowercase().cmp(&b.email.to_lowercase()))
    });

    if !visible_accounts.is_empty() {
        let sep_quick = PredefinedMenuItem::separator(app)?;
        menu_items.push(Box::new(sep_quick));
        let quick_title =
            MenuItem::with_id(app, "quick_switch_title", "快速切号", false, None::<&str>)?;
        menu_items.push(Box::new(quick_title));

        let current_by_platform = ProviderCurrentService::list_current_account_snapshots(&*db)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| {
                item.account_id
                    .filter(|id| !id.trim().is_empty())
                    .map(|id| (account_platform_key(&item.platform), id))
            })
            .collect::<std::collections::HashMap<_, _>>();

        let mut grouped = std::collections::BTreeMap::<String, Vec<IdeAccount>>::new();
        for account in visible_accounts {
            grouped
                .entry(account_platform_key(&account.origin_platform))
                .or_default()
                .push(account);
        }

        for (platform, mut accounts) in grouped {
            accounts.sort_by(|a, b| {
                let a_is_current = current_by_platform
                    .get(&platform)
                    .is_some_and(|id| id == &a.id);
                let b_is_current = current_by_platform
                    .get(&platform)
                    .is_some_and(|id| id == &b.id);
                b_is_current
                    .cmp(&a_is_current)
                    .then_with(|| is_attention_account(a).cmp(&is_attention_account(b)))
                    .then_with(|| a.email.to_lowercase().cmp(&b.email.to_lowercase()))
            });

            let platform_header = MenuItem::with_id(
                app,
                format!("quick_switch_platform_{}", platform),
                format!("{} 账号", platform),
                false,
                None::<&str>,
            )?;
            menu_items.push(Box::new(platform_header));

            let shown = accounts.len().min(QUICK_SWITCH_MAX_PER_PLATFORM);
            for account in accounts.iter().take(QUICK_SWITCH_MAX_PER_PLATFORM) {
                let id = quick_switch_menu_id(&platform, &account.id);
                let label = account_display_label(account);
                let is_current = current_by_platform
                    .get(&platform)
                    .is_some_and(|current_id| current_id == &account.id);
                let item = CheckMenuItem::with_id(app, id, label, true, is_current, None::<&str>)?;
                menu_items.push(Box::new(item));
            }

            if accounts.len() > shown {
                let more_item = MenuItem::with_id(
                    app,
                    format!("quick_switch_more_{}", platform),
                    format!("... 其余 {} 个账号请到主窗口处理", accounts.len() - shown),
                    false,
                    None::<&str>,
                )?;
                menu_items.push(Box::new(more_item));
            }
        }
    } else {
        let sep_quick = PredefinedMenuItem::separator(app)?;
        menu_items.push(Box::new(sep_quick));
        let quick_empty = MenuItem::with_id(
            app,
            "quick_switch_empty",
            "(当前托盘范围内无可切换账号)",
            false,
            None::<&str>,
        )?;
        menu_items.push(Box::new(quick_empty));
    }

    let sep2 = PredefinedMenuItem::separator(app)?;
    menu_items.push(Box::new(sep2));

    let refresh_item =
        MenuItem::with_id(app, "refresh_balances", "刷新全部余额", true, None::<&str>)?;
    menu_items.push(Box::new(refresh_item));

    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    menu_items.push(Box::new(quit_item));

    let item_refs: Vec<&dyn IsMenuItem<Wry>> = menu_items.iter().map(|b| b.as_ref()).collect();
    Menu::with_items(app, &item_refs)
}

pub(super) fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "open_main" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "open_accounts" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                let _ = window.emit("navigate_to_page", "accounts");
            }
        }
        "quit" => {
            app.exit(0);
        }
        "refresh_balances" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("force_refresh_analytics", ());
            }
            app.notification()
                .builder()
                .title("AI Singularity")
                .body("已触发后台余额刷新全量检测_")
                .show()
                .unwrap_or_default();
        }
        _ if id.starts_with("switch_provider_") => {
            let provider_id = id.trim_start_matches("switch_provider_");
            let db = app.state::<Database>();
            let service = ProviderService::new(&*db);

            match service.switch_provider(provider_id) {
                Ok(_) => {
                    if let Ok(providers) = service.list_providers() {
                        if let Some(p) = providers.iter().find(|p| p.id == provider_id) {
                            app.notification()
                                .builder()
                                .title("AI Singularity")
                                .body(format!("✅ 已切换至 Provider: {}", p.name))
                                .show()
                                .unwrap_or_default();
                        }
                    }

                    super::update_tray_menu(app);

                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.emit("provider_switched", provider_id);
                    }
                    EventBus::emit_data_changed(app, "providers", "switch", "tray.switch_provider");
                }
                Err(e) => {
                    app.notification()
                        .builder()
                        .title("切换失败")
                        .body(e.to_string())
                        .show()
                        .unwrap_or_default();
                }
            }
        }
        _ if id.starts_with(QUICK_SWITCH_MENU_PREFIX) => {
            let Some((platform, account_id)) = parse_quick_switch_menu_id(id) else {
                app.notification()
                    .builder()
                    .title("切号失败")
                    .body("托盘菜单项格式异常，请重试")
                    .show()
                    .unwrap_or_default();
                super::update_tray_menu(app);
                return;
            };

            let db = app.state::<Database>();
            let target = db
                .get_all_ide_accounts()
                .unwrap_or_default()
                .into_iter()
                .find(|item| item.id == account_id);

            let Some(account) = target else {
                app.notification()
                    .builder()
                    .title("切号失败")
                    .body("目标账号不存在，托盘已刷新")
                    .show()
                    .unwrap_or_default();
                super::update_tray_menu(app);
                return;
            };

            match IdeInjector::execute_injection(&account) {
                Ok(_) => {
                    crate::commands::floating_account_card::emit_floating_account_changed(
                        app,
                        &account.origin_platform,
                        Some(&account.id),
                        "tray.quick_switch",
                    );
                    EventBus::emit_data_changed(
                        app,
                        "ide_accounts",
                        "force_inject",
                        "tray.quick_switch",
                    );
                    super::update_tray_menu(app);
                    app.notification()
                        .builder()
                        .title("AI Singularity")
                        .body(format!(
                            "✅ 已切换 {} 当前账号：{}",
                            platform,
                            account
                                .label
                                .as_ref()
                                .map(|value| value.trim())
                                .filter(|value| !value.is_empty())
                                .unwrap_or(account.email.as_str())
                        ))
                        .show()
                        .unwrap_or_default();
                }
                Err(err) => {
                    app.notification()
                        .builder()
                        .title("切号失败")
                        .body(err.to_string())
                        .show()
                        .unwrap_or_default();
                }
            }
        }
        _ => {}
    }
}
