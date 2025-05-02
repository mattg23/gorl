use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;

pub type WindowId = usize;

#[derive(Debug)]
pub enum GorlMsg {
    OpenLogWindow,
    CloseLogWindow(WindowId),
    OpenFileIn(WindowId, PathBuf),
}

static WINDOW_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn next_window_id() -> usize {
    WINDOW_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}
