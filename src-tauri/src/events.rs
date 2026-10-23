use std::time::{Duration, Instant};

use baton_core::StatsSnapshot;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::CommandError;
use crate::relay::{Relay, RelayState, Status};
use crate::tray::Tray;
use crate::window::show_main_window;

#[derive(Default)]
pub struct PendingError(pub std::sync::Mutex<Option<CommandError>>);

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsEvent {
    active: u64,
    failed: u64,
    up_rate: u64,
    down_rate: u64,
}

pub fn publish_status(
    app: &AppHandle,
    relay: &Relay,
    result: Result<(), CommandError>,
) -> Result<Status, CommandError> {
    let status = relay.status();
    if let Some(tray) = app.try_state::<Tray>() {
        tray.sync(app, &status);
    }
    let _ = app.emit("relay://status", &status);
    result.map(|_| status)
}

pub fn report_error(app: &AppHandle, error: CommandError) {
    if let Ok(mut pending) = app.state::<PendingError>().0.lock() {
        *pending = Some(error.clone());
    }
    let _ = app.emit("app://error", error);
    show_main_window(app);
}

pub fn spawn_stats_ticker(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        let mut previous: Option<(StatsSnapshot, Instant)> = None;
        loop {
            interval.tick().await;
            let stats = app.state::<RelayState>().lock().await.stats();
            let now = Instant::now();
            let event = stats.map_or_else(StatsEvent::default, |stats| {
                let (last, at) = previous.unwrap_or((stats, now));
                let secs = now.duration_since(at).as_secs_f64().max(0.001);
                StatsEvent {
                    active: stats.active,
                    failed: stats.failed,
                    up_rate: rate(stats.up_bytes, last.up_bytes, secs),
                    down_rate: rate(stats.down_bytes, last.down_bytes, secs),
                }
            });
            previous = stats.map(|stats| (stats, now));
            let _ = app.emit("relay://stats", event);
        }
    });
}

fn rate(now: u64, last: u64, secs: f64) -> u64 {
    (now.saturating_sub(last) as f64 / secs) as u64
}
