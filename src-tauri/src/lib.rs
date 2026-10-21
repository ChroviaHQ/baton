mod config;
mod error;
mod relay;
mod tray;

use std::time::{Duration, Instant};

use baton_core::{EgressInfo, Hop, StatsSnapshot, probe_egress};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tokio::sync::Mutex;

use crate::config::AppConfig;
use crate::error::CommandError;
use crate::relay::{Relay, Status};
use crate::tray::Tray;

const PROBE_TIMEOUT: Duration = Duration::from_secs(12);
const HIDDEN_ARG: &str = "--hidden";

type RelayState = Mutex<Relay>;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsEvent {
    active: u64,
    failed: u64,
    up_rate: u64,
    down_rate: u64,
}

#[tauri::command]
async fn get_status(state: State<'_, RelayState>) -> Result<Status, CommandError> {
    Ok(state.lock().await.status())
}

#[tauri::command]
async fn start_relay(app: AppHandle, state: State<'_, RelayState>) -> Result<Status, CommandError> {
    let mut relay = state.lock().await;
    let result = relay.start().await;
    publish(&app, &relay.status());
    result.map(|_| relay.status())
}

#[tauri::command]
async fn stop_relay(app: AppHandle, state: State<'_, RelayState>) -> Result<Status, CommandError> {
    let mut relay = state.lock().await;
    let result = relay.stop().await;
    publish(&app, &relay.status());
    result.map(|_| relay.status())
}

#[tauri::command]
async fn save_config(
    app: AppHandle,
    state: State<'_, RelayState>,
    config: AppConfig,
) -> Result<Status, CommandError> {
    let mut relay = state.lock().await;
    let show_in_dock = relay.config().show_in_dock;
    let result = relay.update(config).await;
    if relay.config().show_in_dock != show_in_dock {
        apply_dock_visibility(&app, relay.config().show_in_dock);
        show_main_window(&app);
    }
    publish(&app, &relay.status());
    result.map(|_| relay.status())
}

#[tauri::command]
async fn probe(hops: Vec<Hop>) -> Result<EgressInfo, CommandError> {
    for (index, hop) in hops.iter().enumerate() {
        hop.validate()
            .map_err(|error| CommandError::invalid_hop(index, error))?;
    }
    Ok(probe_egress(&hops, PROBE_TIMEOUT).await?)
}

#[tauri::command]
async fn detect_system_proxy(state: State<'_, RelayState>) -> Result<Hop, CommandError> {
    let relay = state.lock().await;
    relay.detect_upstream(relay.config().listen_port)
}

#[tauri::command]
fn get_launch_at_login(app: AppHandle) -> Result<bool, CommandError> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| CommandError::new("launch_at_login_failed", error))
}

#[tauri::command]
fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<bool, CommandError> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|error| CommandError::new("launch_at_login_failed", error))?;
    get_launch_at_login(app)
}

fn publish(app: &AppHandle, status: &Status) {
    if let Some(tray) = app.try_state::<Tray>() {
        tray.sync(app, status);
    }
    let _ = app.emit("relay://status", status);
}

#[cfg(target_os = "macos")]
fn apply_dock_visibility(app: &AppHandle, visible: bool) {
    let policy = if visible {
        tauri::ActivationPolicy::Regular
    } else {
        tauri::ActivationPolicy::Accessory
    };
    let _ = app.set_activation_policy(policy);
    #[cfg(debug_assertions)]
    if visible {
        let _ = app.run_on_main_thread(restore_dev_dock_icon);
    }
}

#[cfg(all(target_os = "macos", debug_assertions))]
fn restore_dev_dock_icon() {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let data = NSData::with_bytes(include_bytes!("../icons/icon.png"));
    if let Some(icon) = NSImage::initWithData(NSImage::alloc(), &data) {
        unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&icon)) };
    }
}

#[cfg(not(target_os = "macos"))]
fn apply_dock_visibility(_app: &AppHandle, _visible: bool) {}

pub(crate) fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub(crate) fn spawn_toggle(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<RelayState>();
        let mut relay = state.lock().await;
        let result = if relay.status().running {
            relay.stop().await
        } else {
            relay.start().await
        };
        publish(&app, &relay.status());
        if let Err(error) = result {
            let _ = app.emit("relay://error", error);
            show_main_window(&app);
        }
    });
}

pub(crate) fn spawn_toggle_system_proxy(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<RelayState>();
        let mut relay = state.lock().await;
        let mut config = relay.config().clone();
        config.system_proxy = !config.system_proxy;
        let result = relay.update(config).await;
        publish(&app, &relay.status());
        if let Err(error) = result {
            let _ = app.emit("relay://error", error);
            show_main_window(&app);
        }
    });
}

fn spawn_stats_ticker(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        let mut previous: Option<(StatsSnapshot, Instant)> = None;
        loop {
            interval.tick().await;
            let stats = app.state::<RelayState>().lock().await.stats();
            let now = Instant::now();
            let event = match stats {
                Some(stats) => {
                    let (up_rate, down_rate) = match previous {
                        Some((last, at)) => {
                            let secs = now.duration_since(at).as_secs_f64().max(0.001);
                            (
                                (stats.up_bytes.saturating_sub(last.up_bytes) as f64 / secs) as u64,
                                (stats.down_bytes.saturating_sub(last.down_bytes) as f64 / secs)
                                    as u64,
                            )
                        }
                        None => (0, 0),
                    };
                    previous = Some((stats, now));
                    StatsEvent {
                        active: stats.active,
                        failed: stats.failed,
                        up_rate,
                        down_rate,
                    }
                }
                None => {
                    previous = None;
                    StatsEvent::default()
                }
            };
            let _ = app.emit("relay://stats", event);
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
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_ARG]),
        ))
        .invoke_handler(tauri::generate_handler![
            get_status,
            start_relay,
            stop_relay,
            save_config,
            probe,
            detect_system_proxy,
            get_launch_at_login,
            set_launch_at_login
        ])
        .setup(|app| {
            let data_dir = app.path().app_config_dir()?;
            let relay = Relay::new(&data_dir);
            relay.recover_system_proxy();
            let auto_connect = relay.config().auto_connect;
            apply_dock_visibility(app.handle(), relay.config().show_in_dock);
            let tray = tray::build(app.handle(), &relay.status())?;
            app.manage(tray);
            app.manage::<RelayState>(Mutex::new(relay));
            spawn_stats_ticker(app.handle().clone());
            #[cfg(not(target_os = "macos"))]
            if let Some(window) = app.get_webview_window("main") {
                window.set_decorations(false)?;
                window.set_shadow(true)?;
            }
            if !std::env::args().any(|arg| arg == HIDDEN_ARG) {
                show_main_window(app.handle());
            }
            if auto_connect {
                spawn_toggle(app.handle().clone());
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
