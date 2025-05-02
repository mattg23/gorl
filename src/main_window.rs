use std::fs::File;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::RwLock;

use crate::highlighter::Highlighter;
use crate::lineview::LineBasedFileView;

use crate::search::SearchWindow;
use fltk::draw;
use fltk::enums;
use fltk::{
    app,
    enums::Event,
    prelude::*,
    table::{Table, TableContext},
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
    window: Option<DoubleWindow>,
    table: Table,
}

impl GorlLogWindow {
    pub fn new() -> Self {
        info!("Creating Gorl Log Window. Settings = {:?}", SETTINGS.read());
        let id = next_window_id();

        let (s, receiver) = app::channel();

        let settings_lck = SETTINGS.read().unwrap();
        let highlight_settings = settings_lck.default_highlights.as_ref();

        let highlighter = Highlighter::new(highlight_settings.map_or(vec![], |a| a.clone()));

        let mut win = window::DoubleWindow::default()
            .with_size(1200, 800)
            .with_label("GORL 🪵🪟");

        let mut table = Table::default().with_size(1200, 800).center_of(&win);
        table.set_rows(0);
        table.set_row_header(true);
        table.set_cols(1);
        table.set_col_header(false);
        table.set_col_resize(true);

        table.set_col_width(0, 1200);

        table.end();

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
                Event::Hide => {
                    outbox.send(GorlMsg::CloseLogWindow(id));
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
        let mut self_ = Self {
            view: Rc::new(RwLock::new(None)),
            search_window: None,
            outbox: s,
            highlighter,
            window: Some(win),
            id,
            table: table.clone(),
        };

        table.draw_cell({
            let outbox = s.clone();
            let mut self_ptr = (&mut self_).clone();
            move |t, ctx, row, col, x, y, w, h| {
                self_ptr.draw_cell(t, ctx, row, col, x, y, w, h);
            }
        });

        self_
    }

    pub fn draw_cell(
        &mut self,
        t: &mut Table,
        ctx: TableContext,
        row: i32,
        col: i32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) {
        //debug!("gorl::main_window::draw_cell: {ctx:?},{row},{col},{x},{y},{w},{h}");
        match ctx {
            TableContext::RowHeader => self.draw_text(
                &Self::fmt_to_row_header((row + 1) as u64),
                x,
                y,
                w,
                h,
                enums::Align::Right,
                enums::Color::ForeGround,
                enums::Color::BackGround,
            ),
            TableContext::Cell => self.draw_data(row, x, y, w, h),
            _ => (),
        }
    }

    fn draw_data(&mut self, row: i32, x: i32, y: i32, w: i32, h: i32) {
        if let Ok(mut lck) = self.view.write() {
            if let Some(view) = lck.as_mut() {
                if let Ok(line) = view.get_line(row as u64) {
                    let mut fg = enums::Color::Foreground;
                    let mut bg = enums::Color::Background;
                    if let Some(highlight) = self.highlighter.matches(&line) {
                        fg = enums::Color::rgb_color(
                            highlight.fg_color.0,
                            highlight.fg_color.1,
                            highlight.fg_color.2,
                        );
                        bg = enums::Color::rgb_color(
                            highlight.bg_color.0,
                            highlight.bg_color.1,
                            highlight.bg_color.2,
                        );
                        debug!("gorl::main_window::draw_data: {line:?} matched highlight settings.")
                    }

                    self.draw_text(line.as_str(), x, y, w, h, enums::Align::Left, fg, bg);
                }
            }
        }
    }

    fn draw_text(
        &self,
        txt: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        txt_align: enums::Align,
        fg: enums::Color,
        bg: enums::Color,
    ) {
        draw::push_clip(x, y, w, h);
        draw::draw_box(enums::FrameType::FlatBox, x, y, w, h, bg);
        draw::set_draw_color(fg);
        draw::set_font(enums::Font::Courier, 14);
        draw::draw_text2(txt, x, y, w, h, txt_align);
        draw::pop_clip();
    }

    pub fn get_id(&self) -> WindowId {
        self.id
    }

    pub fn close(&mut self) {
        info!("close: WinId={}", self.get_id());
        if let Some(mut win) = self.window.clone() {
            self.window = None;
            win.hide();
            app::delete_widget(win);
        }
    }

    fn fmt_to_row_header(l: u64) -> String {
        format!("{l}| ")
    }

    pub fn process_message(&mut self, msg: &GorlMsg) {
        let id = self.get_id();

        info!("process_message: {msg:?}");

        match msg {
            GorlMsg::OpenFileIn(w, path) if *w == id => match self.open_file(path) {
                Ok(view) => {
                    let lc = view.line_count();
                    *self.view.write().unwrap() = Some(view);
                    self.table.set_rows(lc as i32);
                    let (w, _) = draw::measure(Self::fmt_to_row_header(lc).as_str(), false);
                    self.table.set_row_header_width(w);

                    self.window
                        .as_mut()
                        .expect("dropped a file into an hidden window?")
                        .set_label(format!("GORL 🪵🪟 - {path:?}").as_str());
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
