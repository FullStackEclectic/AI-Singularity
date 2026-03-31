use tauri::Manager;

mod commands;
mod db;
mod error;
mod models;
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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Key management
            commands::keys::list_keys,
            commands::keys::add_key,
            commands::keys::delete_key,
            commands::keys::update_key,
            commands::keys::check_key,
            // Balance & usage
            commands::balance::get_all_balances,
            commands::balance::get_platform_balance,
            // Models
            commands::models::list_models,
            commands::models::get_platform_models,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AI Singularity");
}
