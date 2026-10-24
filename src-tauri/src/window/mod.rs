use tauri::{AppHandle, Manager, WebviewWindowBuilder};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(windows)]
use windows as platform;

#[cfg(target_os = "macos")]
pub use macos::{TitleBarMetrics, title_bar_metrics};
pub use platform::apply_dock_visibility;

const MAIN_WINDOW_LABEL: &str = "main";

pub fn create_main_window(app: &AppHandle) -> tauri::Result<()> {
    let window_configs = &app.config().app.windows;
    if let Some(config) = window_configs.iter().find(|config| config.label == MAIN_WINDOW_LABEL) {
        let mut config = config.clone();
        platform::configure_main_window(&mut config);
        WebviewWindowBuilder::from_config(app, &config)?.build()?;
    }
    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        platform::publish_title_bar_metrics(&window);
    }
}
