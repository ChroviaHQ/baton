use std::time::Duration;

use baton_core::{EgressInfo, Hop, probe_egress};
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::RelayState;
use crate::config::AppConfig;
use crate::error::CommandError;
use crate::events::{PendingError, publish};
use crate::relay::Status;
use crate::window::{apply_dock_visibility, show_main_window};

const PROBE_TIMEOUT: Duration = Duration::from_secs(12);

#[tauri::command]
pub async fn get_status(state: State<'_, RelayState>) -> Result<Status, CommandError> {
    Ok(state.lock().await.status())
}

#[tauri::command]
pub async fn start_relay(
    app: AppHandle,
    state: State<'_, RelayState>,
) -> Result<Status, CommandError> {
    let mut relay = state.lock().await;
    let result = relay.start().await;
    publish(&app, &relay, result)
}

#[tauri::command]
pub async fn stop_relay(
    app: AppHandle,
    state: State<'_, RelayState>,
) -> Result<Status, CommandError> {
    let mut relay = state.lock().await;
    let result = relay.stop().await;
    publish(&app, &relay, result)
}

#[tauri::command]
pub async fn save_config(
    app: AppHandle,
    state: State<'_, RelayState>,
    config: AppConfig,
) -> Result<Status, CommandError> {
    let mut relay = state.lock().await;
    let show_in_dock = relay.config().show_in_dock;
    let result = relay.apply_config(config).await;
    if relay.config().show_in_dock != show_in_dock {
        apply_dock_visibility(&app, relay.config().show_in_dock);
        show_main_window(&app);
    }
    publish(&app, &relay, result)
}

#[tauri::command]
pub async fn probe(hops: Vec<Hop>) -> Result<EgressInfo, CommandError> {
    for (index, hop) in hops.iter().enumerate() {
        hop.validate().map_err(|error| CommandError::invalid_hop(index, error))?;
    }
    probe_egress(&hops, PROBE_TIMEOUT).await.map_err(|error| CommandError::probe(error, hops.len()))
}

#[tauri::command]
pub async fn detect_system_proxy(state: State<'_, RelayState>) -> Result<Hop, CommandError> {
    let relay = state.lock().await;
    relay.detect_upstream(relay.config().listen_port)
}

#[tauri::command]
pub fn take_pending_error(pending: State<'_, PendingError>) -> Option<CommandError> {
    pending.0.lock().ok()?.take()
}

#[tauri::command]
pub fn get_launch_at_login(app: AppHandle) -> Result<bool, CommandError> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| CommandError::new("launch_at_login_failed", error))
}

#[tauri::command]
pub fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<bool, CommandError> {
    let manager = app.autolaunch();
    let result = if enabled { manager.enable() } else { manager.disable() };
    result.map_err(|error| CommandError::new("launch_at_login_failed", error))?;
    get_launch_at_login(app)
}
