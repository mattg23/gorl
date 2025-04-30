pub type WindowId = u64;

#[derive(Debug)]
pub enum GorlMsg {
    OpenLogWindow,
    CloseLogWindow(u64),
}
