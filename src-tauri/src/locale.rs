use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize)]
struct Locale {
    tray: Labels,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Labels {
    pub(crate) start: String,
    pub(crate) stop: String,
    pub(crate) system_proxy: String,
    pub(crate) check_update: String,
    pub(crate) checking_update: String,
    pub(crate) up_to_date: String,
    pub(crate) quit: String,
    pub(crate) running: String,
    pub(crate) stopped: String,
}

pub(crate) fn labels() -> &'static Labels {
    static LABELS: OnceLock<Labels> = OnceLock::new();
    LABELS.get_or_init(|| {
        let locale = match sys_locale::get_locale() {
            Some(locale) if locale.starts_with("zh") => include_str!("../../src/locales/zh.json"),
            _ => include_str!("../../src/locales/en.json"),
        };
        serde_json::from_str::<Locale>(locale).expect("bundled locale is valid").tray
    })
}
