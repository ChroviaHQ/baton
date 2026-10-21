use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

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
            match check(&app).await {
                Ok(Some(update)) => {
                    let version = update.version.clone();
                    if let Ok(mut pending) = app.state::<PendingUpdate>().0.lock() {
                        *pending = Some(update);
                    }
                    let _ = app.emit("relay://update", version);
                    break;
                }
                Ok(None) => {}
                Err(error) => tracing::warn!(%error, "update check failed"),
            }
        }
    });
}

async fn check(app: &AppHandle) -> tauri_plugin_updater::Result<Option<Update>> {
    app.updater()?.check().await
}
