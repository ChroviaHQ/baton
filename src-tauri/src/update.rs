use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::CommandError;
use crate::events::report_error;
use crate::locale::labels;
use crate::tray::Tray;
use crate::window::show_main_window;

const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Default)]
pub(crate) struct PendingUpdate(pub(crate) Mutex<Option<Update>>);

pub(crate) fn spawn_update_checker(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let start = tokio::time::Instant::now() + FIRST_CHECK_DELAY;
        let mut interval = tokio::time::interval_at(start, CHECK_INTERVAL);
        loop {
            interval.tick().await;
            if matches!(check(&app).await, Ok(true)) {
                break;
            }
        }
    });
}

pub(crate) fn spawn_manual_update_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let tray = app.state::<Tray>();
        tray.set_checking_update(true);
        let result = check(&app).await;
        tray.set_checking_update(false);
        match result {
            Ok(true) => show_main_window(&app),
            Ok(false) => {
                let version = app.package_info().version.to_string();
                let message = labels().up_to_date.replace("{version}", &version);
                app.dialog().message(message).title("Baton").show(|_| {});
            }
            Err(error) => report_error(&app, CommandError::new("update_check_failed", error)),
        }
    });
}

async fn check(app: &AppHandle) -> tauri_plugin_updater::Result<bool> {
    let pending = app.state::<PendingUpdate>();
    if pending.0.lock().is_ok_and(|update| update.is_some()) {
        return Ok(true);
    }
    let result = match app.updater() {
        Ok(updater) => updater.check().await,
        Err(error) => Err(error),
    };
    let Some(update) = result.inspect_err(|error| tracing::warn!(%error, "update check failed"))?
    else {
        return Ok(false);
    };
    let version = update.version.clone();
    if let Ok(mut pending) = pending.0.lock() {
        *pending = Some(update);
    }
    let _ = app.emit("relay://update", version);
    Ok(true)
}
