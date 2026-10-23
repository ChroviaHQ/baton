use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize)]
struct Locale {
    tray: Labels,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Labels {
    pub start: String,
    pub stop: String,
    pub system_proxy: String,
    pub check_update: String,
    pub checking_update: String,
    pub up_to_date: String,
    pub quit: String,
    pub running: String,
    pub stopped: String,
}

pub fn labels() -> &'static Labels {
    static LABELS: OnceLock<Labels> = OnceLock::new();
    LABELS.get_or_init(|| {
        let locale = match sys_locale::get_locale() {
            Some(locale) if locale.starts_with("zh") => include_str!("../../src/locales/zh.json"),
            _ => include_str!("../../src/locales/en.json"),
        };
        serde_json::from_str::<Locale>(locale).expect("bundled locale is valid").tray
    })
}
