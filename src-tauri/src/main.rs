// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use anyhow::Result;
use log::{error, info, warn};
use tauri::{
    Emitter, LogicalPosition, LogicalSize, Manager, Position, Size, State, WindowEvent,
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;
use tauri_plugin_window_state::{AppHandleExt, StateFlags, WindowExt};

use crate::app::autostart::AutoLaunchManager;

const METER_WINDOW_LABEL: &str = "main";
const METER_MINI_WINDOW_LABEL: &str = "mini";
const LOGS_WINDOW_LABEL: &str = "logs";
const WINDOW_STATE_FLAGS: StateFlags = StateFlags::from_bits_truncate(
    StateFlags::FULLSCREEN.bits()
        | StateFlags::MAXIMIZED.bits()
        | StateFlags::POSITION.bits()
        | StateFlags::SIZE.bits()
        | StateFlags::VISIBLE.bits(),
);

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
        .setup(|app| {
            info!("starting app v{}", app.package_info().version);

            // if let Err(e) = setup_db(app.handle()) {
            //     warn!("error setting up database: {e}");
            // }

            // setup_tray(app.handle())?;

            let app_path = std::env::current_exe()?.display().to_string();
            app.manage(AutoLaunchManager::new(&app.package_info().name, &app_path));

            // let settings = read_settings(app.handle()).ok();

            let meter_window = app.get_webview_window(METER_WINDOW_LABEL).unwrap();
            meter_window
                .restore_state(WINDOW_STATE_FLAGS)
                .expect("failed to restore window state");

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running application");

    Ok(())
}
