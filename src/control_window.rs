use log::{error, info};
use std::sync::Arc;

use crate::SETTINGS;
use fltk::{
    app,
    prelude::*,
    window::{self, SingleWindow},
};

use crate::common::GorlMsg;

#[derive(Clone)]
pub(crate) struct ControlPanel {
    outbox: app::Sender<GorlMsg>,
    mem_label: Option<String>,
}

impl ControlPanel {
    pub fn new() -> Self {
        info!("Creating CTRL Window. Settings = {:?}", SETTINGS.read());

        let (sender, _) = app::channel();

        let mut self_ = Self {
            mem_label: None,
            outbox: sender,
        };
        self_.setup();
        self_
    }

    fn setup(&mut self) {
        let mut ctrl_window = window::SingleWindow::default()
            .with_size(400, 64)
            .with_label("GORL");
        ctrl_window.show();
    }

    fn get_mem_info() -> Option<String> {
        memory_stats::memory_stats()
            .map(|stats| humansize::format_size(stats.virtual_mem, humansize::WINDOWS))
    }
}
