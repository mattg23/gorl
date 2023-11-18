use config::{Config, File};
use log::{error, info};
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct FontSettings {
    pub size: i32,
    pub name: String,
    pub italic: bool,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            size: 8,
            name: "Consolas".to_owned(),
            italic: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub(crate) struct Settings {
    pub font: FontSettings,
    pub cache_size: u64,
    pub file_buffer_mb: usize
}

pub(crate) const DEF_CACHE_RANGE: u64 = 500;
impl Default for Settings {
    fn default() -> Self {
        Self {
            cache_size: DEF_CACHE_RANGE,
            file_buffer_mb: 8,
            font: FontSettings::default(),
        }
    }
}

impl Settings {
    pub fn new() -> Self {
        let s = Config::builder()
            .add_source(config::Environment::with_prefix("GORL"))
            .add_source(File::with_name("settings"))
            .build();

        if let Ok(config) = s {
            match config.try_deserialize() {
                Ok(settings) => {
                    info!("Loaded config");
                    return settings;
                }
                Err(err) => {
                    error!("ERROR deserializing config: {err}")
                }
            }
        } else {
            error!("ERROR building config: {s:?}")
        }

        Settings::default()
    }
}
