use config::{Config, File};
use log::{error, info};
use serde_derive::Deserialize;

use crate::highlighter::HighlightSetting;

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
    pub file_buffer_mb: usize,
    pub max_nb_of_ui_threads: usize,
    pub max_nb_of_lines_to_copy: u32,
    pub default_highlights: Option<Vec<HighlightSetting>>,
    pub keep_search_res_in_mem_until: Option<usize>,
}

pub(crate) const DEF_CACHE_RANGE: u64 = 500;
impl Default for Settings {
    fn default() -> Self {
        Self {
            cache_size: DEF_CACHE_RANGE,
            file_buffer_mb: 8,
            max_nb_of_ui_threads: 64,
            max_nb_of_lines_to_copy: 2500,
            font: FontSettings::default(),
            default_highlights: None,
            keep_search_res_in_mem_until: Some(32 * 1024 * 1024),
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
