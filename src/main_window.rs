use std::fs::File;
use std::rc::Rc;
use std::sync::RwLock;

use crate::highlighter::Highlighter;
use crate::lineview::LineBasedFileView;

use crate::search::SearchWindow;
use fltk::app;
use log::{debug, error, info};

use crate::SETTINGS;

use crate::common::GorlMsg;

#[derive(Copy, Clone, Debug)]
pub(crate) enum MwMessage {
    JumpTo(u64),
}

#[derive(Clone)]
pub(crate) struct GorlLogWindow {
    view: Rc<RwLock<Option<LineBasedFileView<File>>>>,
    search_window: Option<SearchWindow>,
    outbox: app::Sender<GorlMsg>,
    highlighter: Highlighter, //transmitter: Sender<MwMessage>,
}

impl GorlLogWindow {
    pub async fn new() -> Self {
        info!("Creating Main Window. Settings = {:?}", SETTINGS.read());

        let (s, receiver) = app::channel();

        let highlighter = Highlighter::new(vec![]);

        Self {
            view: Rc::new(RwLock::new(None)),
            search_window: None,
            outbox: s,
            highlighter,
        }
    }

    pub fn process_message(msg: &GorlMsg) {}

    fn open_file(&self, path: &str) -> anyhow::Result<LineBasedFileView<File>> {
        let bf = std::time::SystemTime::now();
        let view = LineBasedFileView::new(File::open(path)?)?;
        let now = std::time::SystemTime::now();

        if let Ok(elapsed) = now.duration_since(bf) {
            info!(
                "Indexed {} chunks from {path} in {}s",
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
