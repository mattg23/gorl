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
}

impl ControlPanel {
    pub fn new() -> Self {
        info!("Creating CTRL Window. Settings = {:?}", SETTINGS.read());

        let (sender, _) = app::channel();

        let mut self_ = Self { outbox: sender };
        self_.setup();
        self_
    }

    fn setup(&mut self) {
        let mut ctrl_window = window::SingleWindow::default()
            .with_size(400, 64)
            .with_label("GORL");

        let mut row = fltk::group::Flex::default_fill().row();
        let mut open_window_btn = fltk::button::Button::default().with_label("Open 🪵🪟");
        open_window_btn.set_label_size(18);
        let p_sender = self.outbox.clone();
        open_window_btn.set_callback(move |_| {
            p_sender.send(GorlMsg::OpenLogWindow);
        });
        let mut mem = fltk::frame::Frame::default().with_label("🐏 ");
        let callback = move |handle| {
            if let Some(info) = Self::get_mem_info() {
                mem.set_label(format!("🐏 {info}").as_str());
            }
            app::repeat_timeout3(1.0, handle);
        };
        app::add_timeout3(1.0, callback);
        row.end();
        ctrl_window.end();
        ctrl_window.show();
    }

    pub fn process_msg(&mut self, msg: &GorlMsg) {}

    fn get_mem_info() -> Option<String> {
        memory_stats::memory_stats()
            .map(|stats| humansize::format_size(stats.physical_mem, humansize::WINDOWS))
    }
}
