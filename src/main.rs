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
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::common::{GorlMsg, WindowId};
use crate::main_window::GorlLogWindow;
use fltk::app;
use fltk_theme::WidgetTheme;

lazy_static! {
    static ref SETTINGS: RwLock<settings::Settings> = RwLock::new(settings::Settings::new());
}

struct Gorl {
    app: app::App,
    receiver: app::Receiver<GorlMsg>,
    ctrl: ControlPanel,
    log_windows: HashMap<WindowId, GorlLogWindow>,
}

impl Gorl {
    pub fn new() -> Self {
        let app = app::App::default();
        let (_, receiver) = app::channel();

        app::set_font_size(18);

        let window_theme = WidgetTheme::new(fltk_theme::ThemeType::Dark);
        window_theme.apply();

        let ctrl = ControlPanel::new();
        let log_windows = HashMap::new();

        Self {
            app,
            receiver,
            ctrl,
            log_windows,
        }
    }

    pub fn run(&mut self) {
        while self.app.wait() {
            if let Some(msg) = self.receiver.recv() {
                log::debug!("GORL::RUN:: {msg:?}");
                match msg {
                    GorlMsg::OpenLogWindow => self.open(),
                    GorlMsg::CloseLogWindow(id) => self.close(id),
                    GorlMsg::OpenFileIn(w, _) => {
                        if let Some(win) = self.log_windows.get_mut(&w) {
                            win.process_message(&msg);
                        }
                    }
                }
            }
        }
    }

    fn open(&mut self) {
        let log_win = GorlLogWindow::new();
        self.log_windows.insert(log_win.get_id(), log_win);
    }
    fn close(&mut self, id: WindowId) {
        if let Some((_, mut window)) = self.log_windows.remove_entry(&id) {
            window.close();
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    Gorl::new().run();

    Ok(())
}
