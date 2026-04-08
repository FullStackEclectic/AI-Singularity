use tauri::Manager;

mod atomic_write;
mod commands;
mod db;
mod error;
mod models;
mod proxy;
mod services;
mod store;
mod panic_hook;
pub mod tray;

pub use error::{AppError, AppResult};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--silent"]),
        ))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // 当其他实例试图启动（携带伪协议链接时）触发此回调
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            // 发布包含启动参数（通常是 DeepLink URL）的事件给前端或者其他服务
            use tauri::Emitter;
            let _ = app.emit("deep-link-received", args);
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // 初始化数据库
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");
            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");

            // --- 全局灾难捕获系统 ---
            crate::panic_hook::set_panic_hook(app_data_dir.clone());

            let db_path = app_data_dir.join("data.db");
            let db = db::Database::new(&db_path).expect("Failed to initialize database");

            // --- 主动健康探活守护进程（每 30 分钟探活所有 Key） ---
            crate::services::health_check_daemon::HealthCheckDaemon::new(
                std::sync::Arc::new(db.clone())
            ).start(30);

            // --- WebDAV 配置状态漫游自动同步进程（每 15 分钟） ---
            crate::services::webdav_daemon::WebDavDaemon::new(
                std::sync::Arc::new(db.clone()),
                app_data_dir.clone()
            ).start(15);

            app.manage(db);

            // --- 判断静默启动（托盘后台模式） ---
            let args: Vec<String> = std::env::args().collect();
            let is_silent = args.iter().any(|arg| arg == "--silent");

            if let Some(window) = app.get_webview_window("main") {
                if !is_silent {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            // --- V2.5 高可用防熔断中枢：流控冷却复活引擎 ---
            let app_h_recovery = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                    let db_state = app_h_recovery.state::<crate::db::Database>();
                    db_state.recover_rate_limited_nodes();
                }
            });

            // --- V2.0 系统级守护任务 ---
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    // 每 2 小时轮询一次平台余额快照 (先 Sleep 等待系统稳定运行)
                    std::thread::sleep(std::time::Duration::from_secs(7200));
                    
                    let app_h = app_handle.clone();
                    tauri::async_runtime::block_on(async move {
                        use tauri::Manager;
                        let db_state = app_h.state::<crate::db::Database>();
                        let tracker = crate::services::balance_tracker::BalanceTracker::new(&*db_state);
                        let _ = tracker.refresh_all().await;
                        
                        let alert_service = crate::services::alert::AlertService::new(&*db_state);
                        let alerts = alert_service.get_alerts();
                        alert_service.notify_os_throttle(&app_h, alerts);
                    });
                }
            });

            // 挂载防篡改配置双向同步雷达
            crate::services::watcher::WatcherService::start_watching(app.app_handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Key 管理
            commands::keys::list_keys,
            commands::keys::add_key,
            commands::keys::delete_key,
            commands::keys::update_key,
            commands::keys::check_key,
            // Provider 余额追踪器
            commands::balance_tracker::get_balance_summaries,
            commands::balance_tracker::refresh_provider_balances,
            commands::balance_tracker::refresh_provider_balance,
            commands::balance_tracker::get_balance_history,
            commands::balance_tracker::get_burn_rate_forecast,
            // 统计
            commands::stats::get_dashboard_stats,
            commands::stats::get_token_usage_stats,
            // 代理
            commands::proxy::start_proxy,
            commands::proxy::get_proxy_status,
            commands::proxy::sync_proxy_engine_config,
            // 风控与安全防火墙
            commands::security::get_ip_access_logs,
            commands::security::clear_ip_access_logs,
            commands::security::get_ip_rules,
            commands::security::add_ip_rule,
            commands::security::delete_ip_rule,
            commands::security::toggle_ip_rule,
            // Providers & MCP
            commands::provider::get_providers,
            commands::provider::add_provider,
            commands::provider::update_provider,
            commands::provider::switch_provider,
            commands::provider::delete_provider,
            commands::provider::update_providers_order,
            commands::mcp::get_mcps,
            commands::mcp::add_mcp,
            commands::mcp::toggle_mcp,
            commands::mcp::delete_mcp,
            // Prompts
            commands::prompts::get_prompts,
            commands::prompts::save_prompt,
            commands::prompts::delete_prompt,
            commands::prompts::sync_prompt,
            commands::prompts::sync_prompt_to_tool,
            // 告警 & 测速
            commands::alert::get_alerts,
            commands::speedtest::run_speedtest,
            commands::stream_check::stream_check_provider,
            // 配置备份
            commands::backup::export_config,
            commands::backup::import_config,
            commands::webdav::webdav_test_connection,
            commands::webdav::webdav_save_config,
            commands::webdav::webdav_push,
            commands::webdav::webdav_pull,
            // Skills
            commands::skill::list_skills,
            commands::skill::install_skill,
            commands::skill::update_skill,
            commands::skill::uninstall_skill,
            // Sessions
            commands::session::list_sessions,
            commands::session::get_session_details,
            commands::session::scan_zombies,
            commands::session::launch_session_terminal,
            // OAuth
            commands::oauth::start_oauth_flow,
            commands::oauth::poll_oauth_login,
            commands::oauth::cancel_oauth_flow,
            commands::oauth::prepare_oauth_url,

            // 系统检测
            commands::env::check_system_env_conflicts,
            // 降维指纹核心库 (Ide Account Pool)
            commands::ide_account::get_all_ide_accounts,
            commands::ide_account::import_ide_accounts,
            commands::ide_account::delete_ide_account,
            commands::ide_account::update_ide_account_tags,
            commands::ide_account::update_api_key_tags,
            // 本地 IDE 账号扫描器
            commands::ide_scanner::scan_ide_accounts_from_local,
            commands::ide_scanner::import_from_custom_db,
            commands::ide_scanner::import_v1_accounts,
            // 重装沙盒启动器
            commands::sandbox::launch_tool_sandboxed,
            // 局域网兵工厂分发
            commands::tools::check_tool_status,
            commands::tools::deploy_tool,
            // 洗脑芯片 (强连)
            commands::injector::force_inject_ide,
            // SaaS 分发管理 (User Tokens)
            commands::user_token::create_user_token,
            commands::user_token::get_all_user_tokens,
            commands::user_token::update_user_token,
            commands::user_token::delete_user_token,
            commands::user_token::get_user_token_summary,
            // 监控大盘数据
            commands::analytics::get_dashboard_metrics,
            // 模型重映射 (Model Mappings)
            crate::services::model_mapping::list_model_mappings,
            crate::services::model_mapping::upsert_model_mapping,
            crate::services::model_mapping::delete_model_mapping,
            // 边缘内网穿透 (Cloudflared Tunnel)
            crate::services::tunnel::start_tunnel,
            crate::services::tunnel::stop_tunnel,
            crate::services::tunnel::filter_tunnel_status,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 防止默认的完全关闭行为
                api.prevent_close();
                // 仅隐藏窗口
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running AI Singularity");
}
