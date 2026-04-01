use tauri::Manager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};

mod atomic_write;
mod commands;
mod db;
mod error;
mod models;
mod proxy;
mod services;
mod store;

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

            // 系统托盘
            let open_item = MenuItem::with_id(app, "open", "打开主界面", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Key 管理
            commands::keys::list_keys,
            commands::keys::add_key,
            commands::keys::delete_key,
            commands::keys::update_key,
            commands::keys::check_key,
            // 余额 & 用量
            commands::balance::get_all_balances,
            commands::balance::get_platform_balance,
            commands::balance::refresh_all_balances,
            // 模型
            commands::models::list_models,
            commands::models::get_platform_models,
            // 统计
            commands::stats::get_dashboard_stats,
            // 代理
            commands::proxy::start_proxy,
            commands::proxy::get_proxy_status,
            // Providers & MCP
            commands::provider::get_providers,
            commands::provider::add_provider,
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
            commands::speedtest::run_speedtest,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AI Singularity");
}
