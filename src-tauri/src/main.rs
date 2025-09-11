// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod live;

use anyhow::Result;
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager, State, menu::MenuBuilder};
use tauri_plugin_window_state::{StateFlags, WindowExt};

use crate::app::autostart::AutoLaunchManager;

const METER_WINDOW_LABEL: &str = "main";
const LOGS_WINDOW_LABEL: &str = "logs";
const WINDOW_STATE_FLAGS: StateFlags = StateFlags::from_bits_truncate(
    StateFlags::FULLSCREEN.bits()
        | StateFlags::MAXIMIZED.bits()
        | StateFlags::POSITION.bits()
        | StateFlags::SIZE.bits()
        | StateFlags::VISIBLE.bits(),
);

struct AlwaysOnTop(AtomicBool);
struct ClickThrough(AtomicBool);
struct DebugMode(AtomicBool);

#[tokio::main]
async fn main() -> Result<()> {
    app::init();

    std::panic::set_hook(Box::new(|info| {
        error!("Panicked: {info:?}");

        app::get_logger().unwrap().flush();
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {}))
        // .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(WINDOW_STATE_FLAGS)
                .build(),
        )
        .manage(AlwaysOnTop(AtomicBool::new(true)))
        .manage(ClickThrough(AtomicBool::new(false)))
        .invoke_handler(tauri::generate_handler![
            toggle_always_on_top,
            toggle_clickthrough,
        ])
        .setup(|app| {
            info!("starting app v{}", app.package_info().version);

            // if let Err(e) = setup_db(app.handle()) {
            //     warn!("error setting up database: {e}");
            // }

            let app_path = std::env::current_exe()?.display().to_string();
            app.manage(AutoLaunchManager::new(&app.package_info().name, &app_path));

            // let settings = read_settings(app.handle()).ok();

            let meter_window = app.get_webview_window(METER_WINDOW_LABEL).unwrap();
            meter_window
                .restore_state(WINDOW_STATE_FLAGS)
                .expect("failed to restore window state");

            let logs_window = app.get_webview_window(LOGS_WINDOW_LABEL).unwrap();
            logs_window
                .restore_state(WINDOW_STATE_FLAGS)
                .expect("failed to restore window state");

            // Setup system tray
            setup_tray(app);

            // Start meter-core synchronously
            let app_handle = app.handle().clone();
            tokio::task::spawn(async move {
                if let Err(e) = live::start_sync(app_handle).await {
                    error!("Failed to start Meter Core: {}", e);
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                info!("Window close requested: {}", window.label());

                // Prevent the window from being destroyed
                api.prevent_close();

                // Hide the window instead of closing it
                let _ = window.hide();

                info!("Window hidden instead of closed: {}", window.label());
            }
            tauri::WindowEvent::Destroyed => {
                info!("Window destroyed");

                // Check if this was the last window
                let app_handle = window.app_handle();
                let windows = app_handle.webview_windows();
                let remaining_windows = windows.len();

                info!("Remaining windows after destroy: {}", remaining_windows);

                if remaining_windows == 0 {
                    info!("All windows closed, initiating cleanup...");
                    let app_handle = app_handle.clone();
                    tokio::task::spawn(async move {
                        cleanup_on_shutdown().await;
                        // Exit the application after cleanup
                        app_handle.exit(0);
                    });
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running application");

    Ok(())
}

fn setup_live(app: &tauri::App) {
    let app = app.app_handle().clone();
    tokio::task::spawn_blocking(move || {
        // only start listening when there's no update, otherwise unable to remove driver
        // while !update_checked.load(Ordering::Relaxed) {
        //     std::thread::sleep(std::time::Duration::from_millis(100));
        // }

        live::start(app).map_err(|e| {
            error!("unexpected error occurred in parser: {e}");
        })
    });
}

async fn cleanup_on_shutdown() {
    info!("Application is shutting down, cleaning up meter-core...");

    if let Err(e) = live::stop().await {
        error!("Error during meter-core cleanup: {}", e);
    }

    // Additional WinDivert cleanup if needed

    info!("Cleanup completed");
}

#[tauri::command]
fn toggle_always_on_top(window: tauri::Window, state: State<AlwaysOnTop>) {
    let always_on_top = &state.0;
    let new_state = !always_on_top.load(Ordering::Acquire);
    always_on_top.store(new_state, Ordering::Release);
    window.set_always_on_top(new_state).unwrap();
    let _ = window.emit("on-pinned", new_state);
}

#[tauri::command]
fn toggle_clickthrough(app: tauri::AppHandle, state: State<ClickThrough>) {
    let clickthrough = &state.0;
    let new_state = !clickthrough.load(Ordering::Acquire);
    clickthrough.store(new_state, Ordering::Release);

    // Update main window
    if let Some(meter_window) = app.get_webview_window(METER_WINDOW_LABEL) {
        meter_window.set_ignore_cursor_events(new_state).unwrap();
        let _ = meter_window.emit("on-clickthrough", new_state);
    }

    info!("Clickthrough toggled to: {}", new_state);
}

fn setup_tray(app: &tauri::App) {
    // Setup system tray menu for the tray icon configured in tauri.conf.json
    let menu = create_tray_menu(&app.handle());

    // Set menu for the tray icon (configured in tauri.conf.json)
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_menu(Some(menu));
        let _ = tray.on_menu_event(move |app, event| {
            match event.id().as_ref() {
                "open_meter" => {
                    if let Some(window) = app.get_webview_window(METER_WINDOW_LABEL) {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "open_logs" => {
                    if let Some(window) = app.get_webview_window(LOGS_WINDOW_LABEL) {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "always_on_top" => {
                    // Toggle always on top state
                    let always_on_top_state = app.state::<AlwaysOnTop>();
                    let new_state = !always_on_top_state.0.load(Ordering::Acquire);
                    always_on_top_state.0.store(new_state, Ordering::Release);

                    // Update main window
                    if let Some(window) = app.get_webview_window(METER_WINDOW_LABEL) {
                        let _ = window.set_always_on_top(new_state);
                        let _ = window.emit("on-pinned", new_state);
                    }

                    // Update tray menu
                    let updated_menu = create_tray_menu(app);
                    if let Some(tray) = app.tray_by_id("main") {
                        let _ = tray.set_menu(Some(updated_menu));
                    }

                    info!("Always on top toggled to: {}", new_state);
                }
                "toggle_clickthrough" => {
                    // Toggle clickthrough state
                    let clickthrough_state = app.state::<ClickThrough>();
                    let new_state = !clickthrough_state.0.load(Ordering::Acquire);
                    clickthrough_state.0.store(new_state, Ordering::Release);

                    // Update main window
                    if let Some(meter_window) = app.get_webview_window(METER_WINDOW_LABEL) {
                        meter_window.set_ignore_cursor_events(new_state).unwrap();
                        let _ = meter_window.emit("on-clickthrough", new_state);
                    }

                    // Update tray menu
                    let updated_menu = create_tray_menu(app);
                    if let Some(tray) = app.tray_by_id("main") {
                        let _ = tray.set_menu(Some(updated_menu));
                    }

                    info!("Clickthrough toggled to: {}", new_state);
                }
                "reset_windows" => {
                    // Reset window positions/sizes
                    info!("Windows reset");
                }
                "quit" => {
                    info!("Quit requested from system tray, initiating cleanup...");
                    let app_handle = app.clone();
                    tokio::task::spawn(async move {
                        cleanup_on_shutdown().await;
                        // Exit the application after cleanup
                        app_handle.exit(0);
                    });
                }
                _ => {}
            }
        });
    }
}

fn create_tray_menu(app: &tauri::AppHandle) -> tauri::menu::Menu<tauri::Wry> {
    let always_on_top_state = app.state::<AlwaysOnTop>();
    let always_on_top_text = if always_on_top_state.0.load(Ordering::Acquire) {
        "Always on top ✓"
    } else {
        "Always on top"
    };

    let clickthrough_state = app.state::<ClickThrough>();
    let clickthrough_text = if clickthrough_state.0.load(Ordering::Acquire) {
        "Clickthrough ✓"
    } else {
        "Clickthrough"
    };

    MenuBuilder::new(app)
        .text("open_meter", "Open Meter")
        .text("open_logs", "Open Logs")
        .text("always_on_top", always_on_top_text)
        .text("toggle_clickthrough", clickthrough_text)
        .text("reset_windows", "Reset Windows")
        .separator()
        .text("quit", "Quit")
        .build()
        .unwrap()
}
