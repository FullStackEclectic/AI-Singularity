use tauri::Manager;

mod atomic_write;
mod commands;
mod db;
mod error;
mod models;
mod proxy;
mod services;
mod store;
pub mod tray;

pub use error::{AppError, AppResult};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // 初始化数据库
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");
            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");

            let db_path = app_data_dir.join("data.db");
            let db = db::Database::new(&db_path).expect("Failed to initialize database");
            app.manage(db);

            let _ = crate::tray::setup_tray(app.app_handle());

            // --- V2.0 系统级守护任务 ---
            let app_handle = app.app_handle().clone();
            tokio::spawn(async move {
                // 每 2 小时轮询一次平台余额快照
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(7200));
                use tauri::Manager;
                loop {
                    interval.tick().await;
                    
                    let db_state = app_handle.state::<crate::db::Database>();
                    let tracker = crate::services::balance_tracker::BalanceTracker::new(&*db_state);
                    let _ = tracker.refresh_all().await;
                    
                    let alert_service = crate::services::alert::AlertService::new(&*db_state);
                    let alerts = alert_service.get_alerts();
                    alert_service.notify_os_throttle(&app_handle, alerts);
                }
            });

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
            // 代理
            commands::proxy::start_proxy,
            commands::proxy::get_proxy_status,
            // Providers & MCP
            commands::provider::get_providers,
            commands::provider::add_provider,
            commands::provider::update_provider,
            commands::provider::switch_provider,
            commands::provider::delete_provider,
            commands::mcp::get_mcps,
            commands::mcp::add_mcp,
            commands::mcp::toggle_mcp,
            commands::mcp::delete_mcp,
            // Prompts
            commands::prompts::get_prompts,
            commands::prompts::save_prompt,
            commands::prompts::delete_prompt,
            commands::prompts::sync_prompt,
            // 告警 & 测速
            commands::alert::get_alerts,
            // 配置备份
            commands::backup::export_config,
            commands::backup::import_config,
            // Skills
            commands::skill::list_skills,
            commands::skill::install_skill,
            commands::skill::update_skill,
            commands::skill::uninstall_skill,
            // Sessions
            commands::session::list_sessions,
            commands::session::get_session_details,
            // OAuth
            commands::oauth::start_oauth_flow,
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
