use std::fs::File;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::RwLock;

use crate::highlighter::Highlighter;
use crate::lineview::LineBasedFileView;

use crate::search::SearchWindow;
use fltk::{
    app, button,
    enums::Event,
    prelude::*,
    window::{self, DoubleWindow},
};
use log::{debug, error, info};

use crate::SETTINGS;

use crate::common::{GorlMsg, WindowId, next_window_id};

#[derive(Copy, Clone, Debug)]
pub(crate) enum MwMessage {
    JumpTo(u64),
}

#[derive(Clone)]
pub(crate) struct GorlLogWindow {
    view: Rc<RwLock<Option<LineBasedFileView<File>>>>,
    search_window: Option<SearchWindow>,
    outbox: app::Sender<GorlMsg>,
    id: WindowId,
    highlighter: Highlighter, //transmitter: Sender<MwMessage>,
    window: DoubleWindow,
}

impl GorlLogWindow {
    pub fn new() -> Self {
        info!("Creating Gorl Log Window. Settings = {:?}", SETTINGS.read());
        let id = next_window_id();

        let (s, receiver) = app::channel();

        let highlighter = Highlighter::new(vec![]);

        let mut win = window::DoubleWindow::default()
            .with_size(1200, 800)
            .with_label("GORL 🪵🪟");

        win.end();
        win.make_resizable(true);
        win.show();

        win.handle({
            let mut dnd = false;
            let mut released = false;
            let outbox = s.clone();
            move |_, ev| match ev {
                Event::DndEnter => {
                    dnd = true;
                    true
                }
                Event::DndDrag => true,
                Event::DndRelease => {
                    released = true;
                    true
                }
                Event::Paste => {
                    if dnd && released {
                        let path = app::event_text();
                        let path = path.trim();
                        let path = path.replace("file://", "");
                        let path = std::path::PathBuf::from(&path);
                        if path.exists() {
                            // we use a timeout to avoid pasting the path into the buffer
                            outbox.send(GorlMsg::OpenFileIn(id, path));
                        }
                        dnd = false;
                        released = false;
                        true
                    } else {
                        false
                    }
                }
                Event::DndLeave => {
                    dnd = false;
                    released = false;
                    true
                }
                _ => false,
            }
        });
        Self {
            view: Rc::new(RwLock::new(None)),
            search_window: None,
            outbox: s,
            highlighter,
            window: win,
            id,
        }
    }

    pub fn get_id(&self) -> WindowId {
        self.id
    }

    pub fn close(&mut self) {
        self.window.hide();
    }

    pub fn process_message(&mut self, msg: &GorlMsg) {
        let id = self.get_id();
        match msg {
            GorlMsg::OpenFileIn(w, path) if *w == id => match self.open_file(path) {
                Ok(view) => {
                    *self.view.write().unwrap() = Some(view);
                }
                Err(e) => {
                    error!("could not open {path:?}. ERR={e:?}");
                }
            },
            _ => {}
        };
    }

    fn open_file(&self, path: &PathBuf) -> anyhow::Result<LineBasedFileView<File>> {
        let bf = std::time::SystemTime::now();
        let view = LineBasedFileView::new(File::open(path)?)?;
        let now = std::time::SystemTime::now();

        if let Ok(elapsed) = now.duration_since(bf) {
            info!(
                "Indexed {} chunks from {path:?} in {}s",
                view.page_count(),
                elapsed.as_secs_f64()
            );
        }

        Ok(view)
    }

    fn jump_to(&self, line: u64) {
        debug!("MAIN WINDOW: RECEIVED SEARCH RESULT SELECTED {line}");
    }
}
