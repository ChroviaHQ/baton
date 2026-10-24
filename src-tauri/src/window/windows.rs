use tauri::utils::config::WindowConfig;
use tauri::{AppHandle, WebviewWindow};

pub(super) fn configure_main_window(config: &mut WindowConfig) {
    config.decorations = false;
    config.shadow = true;
}

pub(super) fn publish_title_bar_metrics(_window: &WebviewWindow) {}

pub fn apply_dock_visibility(_app: &AppHandle, _visible: bool) {}
