mod common;
mod control_window;
mod highlighter;
mod lineview;
mod main_window;
mod search;
mod settings;
mod utils;

use crate::control_window::ControlPanel;
use lazy_static::lazy_static;
use std::sync::{Arc, RwLock};

use crate::common::{GorlMsg, WindowId};
use fltk::app;

lazy_static! {
    static ref SETTINGS: RwLock<settings::Settings> = RwLock::new(settings::Settings::new());
}

struct Gorl {
    app: app::App,
    receiver: app::Receiver<GorlMsg>,
    ctrl: ControlPanel,
}

impl Gorl {
    pub fn new() -> Self {
        let app = app::App::default();
        let (_, receiver) = app::channel();

        let ctrl = ControlPanel::new();

        Self {
            app,
            receiver,
            ctrl,
        }
    }

    pub fn run(&mut self) {
        while self.app.wait() {}
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    Gorl::new().run();

    Ok(())
}
