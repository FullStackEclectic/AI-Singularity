use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

mod menu;
mod scope;
#[cfg(test)]
mod tests;
mod types;

use self::menu::{build_menu, handle_menu_event};
use self::scope::{get_scope_platforms, set_scope_platforms};
use self::types::TRAY_ID;

pub fn get_tray_platform_scope(app: &AppHandle) -> Vec<String> {
    get_scope_platforms(app)
}

pub fn set_tray_platform_scope(
    app: &AppHandle,
    platforms: Vec<String>,
) -> Result<Vec<String>, String> {
    let normalized = set_scope_platforms(app, platforms)?;
    update_tray_menu(app);
    Ok(normalized)
}

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_menu(app)?;

    match app.tray_by_id(TRAY_ID) {
        Some(tray) => {
            let _ = tray.set_menu(Some(menu));
        }
        None => {
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

pub fn update_tray_menu(app: &AppHandle) {
    if let Err(e) = setup_tray(app) {
        tracing::error!("更新托盘菜单失败: {}", e);
    }
}
