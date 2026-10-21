use std::time::{Duration, Instant};

use baton_core::StatsSnapshot;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::RelayState;
use crate::error::CommandError;
use crate::relay::Status;
use crate::tray::Tray;
use crate::window::show_main_window;

#[derive(Default)]
pub(crate) struct PendingError(pub(crate) std::sync::Mutex<Option<CommandError>>);

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsEvent {
    active: u64,
    failed: u64,
    up_rate: u64,
    down_rate: u64,
}

pub(crate) fn broadcast_status(app: &AppHandle, status: &Status) {
    if let Some(tray) = app.try_state::<Tray>() {
        tray.sync(app, status);
    }
    let _ = app.emit("relay://status", status);
}

pub(crate) fn report_error(app: &AppHandle, error: CommandError) {
    if let Ok(mut pending) = app.state::<PendingError>().0.lock() {
        *pending = Some(error.clone());
    }
    let _ = app.emit("relay://error", error);
    show_main_window(app);
}

pub(crate) fn spawn_stats_ticker(app: AppHandle) {
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
                    StatsEvent { active: stats.active, failed: stats.failed, up_rate, down_rate }
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
