mod commands;
mod config;
mod error;
mod events;
mod relay;
mod tray;
mod window;

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tokio::sync::Mutex;

use crate::events::{PendingError, broadcast_status, report_error, spawn_stats_ticker};
use crate::relay::Relay;
use crate::window::{apply_dock_visibility, prepare_main_window, show_main_window};

const HIDDEN_ARG: &str = "--hidden";

pub(crate) type RelayState = Mutex<Relay>;

pub(crate) fn spawn_toggle_relay(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<RelayState>();
        let mut relay = state.lock().await;
        let result = if relay.status().running { relay.stop().await } else { relay.start().await };
        broadcast_status(&app, &relay.status());
        if let Err(error) = result {
            report_error(&app, error);
        }
    });
}

pub(crate) fn spawn_toggle_system_proxy(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<RelayState>();
        let mut relay = state.lock().await;
        let mut config = relay.config().clone();
        config.use_as_system_proxy = !config.use_as_system_proxy;
        let result = relay.apply_config(config).await;
        broadcast_status(&app, &relay.status());
        if let Err(error) = result {
            report_error(&app, error);
        }
    });
}

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
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::start_relay,
            commands::stop_relay,
            commands::save_config,
            commands::probe,
            commands::detect_system_proxy,
            commands::take_pending_error,
            commands::get_launch_at_login,
            commands::set_launch_at_login
        ])
        .setup(|app| {
            let data_dir = app.path().app_config_dir()?;
            let relay = Relay::new(&data_dir);
            relay.recover_system_proxy();
            let start_on_launch = relay.config().start_on_launch;
            apply_dock_visibility(app.handle(), relay.config().show_in_dock);
            let tray = tray::build(app.handle(), &relay.status())?;
            app.manage(tray);
            app.manage::<RelayState>(Mutex::new(relay));
            app.manage(PendingError::default());
            spawn_stats_ticker(app.handle().clone());
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
            if let Some(state) = app.try_state::<RelayState>() {
                state.blocking_lock().shutdown_blocking();
            }
        }
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => show_main_window(app),
        _ => {}
    });
}
