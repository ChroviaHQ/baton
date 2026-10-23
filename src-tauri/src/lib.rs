mod commands;
mod config;
mod error;
mod events;
mod locale;
mod relay;
mod tray;
mod update;
mod window;

use baton_core::SystemProxy;
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tokio::sync::Mutex;

use crate::commands::spawn_toggle_relay;
use crate::events::{PendingError, spawn_stats_ticker};
use crate::relay::{Relay, RelayState};
use crate::tray::Tray;
use crate::update::{PendingUpdate, spawn_update_checker};
use crate::window::{apply_dock_visibility, prepare_main_window, show_main_window};

const HIDDEN_ARG: &str = "--hidden";

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "baton=info,baton_core=info".into()),
        )
        .init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![HIDDEN_ARG])))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::start_relay,
            commands::stop_relay,
            commands::save_config,
            commands::probe_egress,
            commands::detect_system_proxy,
            #[cfg(target_os = "macos")]
            commands::get_title_bar_metrics,
            commands::take_pending_error,
            commands::get_launch_at_login,
            commands::set_launch_at_login,
            commands::get_pending_update,
            commands::install_update
        ])
        .setup(|app| {
            let data_dir = app.path().app_config_dir()?;
            let relay = Relay::new(&data_dir);
            relay.recover_system_proxy();
            let start_on_launch = relay.config().start_on_launch;
            apply_dock_visibility(app.handle(), relay.config().show_in_dock);
            let tray = Tray::new(app.handle(), &relay.status())?;
            app.manage(tray);
            app.manage(relay.system_proxy().clone());
            app.manage::<RelayState>(Mutex::new(relay));
            app.manage(PendingError::default());
            app.manage(PendingUpdate::default());
            spawn_stats_ticker(app.handle().clone());
            spawn_update_checker(app.handle().clone());
            prepare_main_window(app.handle())?;
            if !std::env::args().any(|arg| arg == HIDDEN_ARG) {
                show_main_window(app.handle());
            }
            if start_on_launch {
                spawn_toggle_relay(app.handle().clone());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to build Baton");

    app.run(|app, event| match event {
        RunEvent::Exit => {
            if let Some(system_proxy) = app.try_state::<SystemProxy>() {
                if let Err(error) = system_proxy.restore() {
                    tracing::error!(%error, "cannot restore system proxy on exit");
                }
            }
        }
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => show_main_window(app),
        _ => {}
    });
}
