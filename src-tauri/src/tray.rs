use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_notification::NotificationExt;

use crate::db::Database;
use crate::services::provider::ProviderService;

const TRAY_ID: &str = "main_tray";

/// 初始化和更新托盘
pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_menu(app)?;

    match app.tray_by_id(TRAY_ID) {
        Some(tray) => {
            // 已有托盘，更新菜单
            let _ = tray.set_menu(Some(menu));
        }
        None => {
            // 首次创建托盘
            if let Some(icon) = app.default_window_icon().cloned() {
                let _tray = TrayIconBuilder::with_id(TRAY_ID)
                    .menu(&menu)
                    .icon(icon)
                    .tooltip("AI Singularity 控制中心")
                    .on_menu_event(|app: &AppHandle, event: tauri::menu::MenuEvent| {
                        handle_menu_event(app, event.id().as_ref());
                    })
                    .on_tray_icon_event(
                        |tray: &tauri::tray::TrayIcon, event: tauri::tray::TrayIconEvent| {
                            if let TrayIconEvent::Click {
                                button: MouseButton::Left,
                                button_state: MouseButtonState::Up,
                                ..
                            } = event
                            {
                                if let Some(window) = tray.app_handle().get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        },
                    )
                    .build(app)?;
            }
        }
    }

    Ok(())
}

/// 全局刷新托盘菜单（当 Provider 增删改时调用）
pub fn update_tray_menu(app: &AppHandle) {
    if let Err(e) = setup_tray(app) {
        tracing::error!("更新托盘菜单失败: {}", e);
    }
}

/// 动态构建托盘菜单
fn build_menu(app: &AppHandle) -> Result<Menu<Wry>, tauri::Error> {
    let mut menu_items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::new();

    // -- 固定项 --
    let open_item = MenuItem::with_id(app, "open_main", "打开主界面", true, None::<&str>)?;
    menu_items.push(Box::new(open_item));

    let sep1 = PredefinedMenuItem::separator(app)?;
    menu_items.push(Box::new(sep1));

    // -- 动态 Provider 列表 --
    let db = app.state::<Database>();
    let providers = ProviderService::new(&*db)
        .list_providers()
        .unwrap_or_default();

    if providers.is_empty() {
        let empty_item = MenuItem::with_id(
            app,
            "no_provider",
            "(暂无 Provider 配置)",
            false,
            None::<&str>,
        )?;
        menu_items.push(Box::new(empty_item));
    } else {
        for p in providers {
            let id = format!("switch_provider_{}", p.id);
            // 名字：[官方] DeepSeek 等...
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

    // -- 底部分隔线与杂项 --
    let sep2 = PredefinedMenuItem::separator(app)?;
    menu_items.push(Box::new(sep2));

    let refresh_item =
        MenuItem::with_id(app, "refresh_balances", "刷新全部余额", true, None::<&str>)?;
    menu_items.push(Box::new(refresh_item));

    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    menu_items.push(Box::new(quit_item));

    // 转换为借用数组
    let item_refs: Vec<&dyn IsMenuItem<Wry>> = menu_items.iter().map(|b| b.as_ref()).collect();
    Menu::with_items(app, &item_refs)
}

/// 处理菜单点击事件
fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "open_main" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "quit" => {
            app.exit(0);
        }
        "refresh_balances" => {
            // 触发余额刷新，可以通过事件通知前端去调接口，或者这里直接调用后台服务
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
                    // 发桌面系统通知
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

                    // 通过重新 build 菜单打勾
                    update_tray_menu(app);

                    // 通知前端热更新 Provider 列表
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.emit("provider_switched", provider_id);
                    }
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
        _ => {}
    }
}
