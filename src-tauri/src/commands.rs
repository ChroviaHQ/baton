use std::time::Duration;

use baton_core::{EgressInfo, Hop};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

use crate::config::AppConfig;
use crate::error::CommandError;
use crate::events::{PendingError, publish_status, report_error};
use crate::relay::{RelayState, Status, validate_chain};
use crate::update::PendingUpdate;
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
    publish_status(&app, &relay, result)
}

#[tauri::command]
pub async fn stop_relay(
    app: AppHandle,
    state: State<'_, RelayState>,
) -> Result<Status, CommandError> {
    let mut relay = state.lock().await;
    let result = relay.stop().await;
    publish_status(&app, &relay, result)
}

pub fn spawn_toggle_relay(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<RelayState>();
        let mut relay = state.lock().await;
        let result = if relay.is_running() { relay.stop().await } else { relay.start().await };
        if let Err(error) = publish_status(&app, &relay, result) {
            report_error(&app, error);
        }
    });
}

pub fn spawn_toggle_system_proxy(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<RelayState>();
        let mut relay = state.lock().await;
        let mut config = relay.config().clone();
        config.use_as_system_proxy = !config.use_as_system_proxy;
        let result = relay.apply_config(config).await;
        if let Err(error) = publish_status(&app, &relay, result) {
            report_error(&app, error);
        }
    });
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
    publish_status(&app, &relay, result)
}

#[tauri::command]
pub async fn probe_egress(hops: Vec<Hop>) -> Result<EgressInfo, CommandError> {
    validate_chain(&hops)?;
    baton_core::probe_egress(&hops, PROBE_TIMEOUT)
        .await
        .map_err(|error| CommandError::probe(error, hops.len()))
}

#[tauri::command]
pub async fn detect_system_proxy(state: State<'_, RelayState>) -> Result<Hop, CommandError> {
    let relay = state.lock().await;
    relay.detect_system_proxy(relay.config().listen_port)
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn get_title_bar_metrics(
    window: tauri::WebviewWindow,
) -> Option<crate::window::TitleBarMetrics> {
    crate::window::title_bar_metrics(&window)
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

#[tauri::command]
pub fn get_pending_update(pending: State<'_, PendingUpdate>) -> Option<String> {
    pending.0.lock().ok()?.as_ref().map(|update| update.version.clone())
}

#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    state: State<'_, RelayState>,
    pending: State<'_, PendingUpdate>,
) -> Result<(), CommandError> {
    let update = pending
        .0
        .lock()
        .ok()
        .and_then(|pending| pending.clone())
        .ok_or_else(|| update_failed("no update available"))?;
    let bytes = update.download(|_, _| {}, || {}).await.map_err(update_failed)?;
    stop_relay(app.clone(), state).await?;
    update.install(bytes).map_err(update_failed)?;
    app.request_restart();
    Ok(())
}

fn update_failed(error: impl ToString) -> CommandError {
    CommandError::new("update_failed", error)
}
