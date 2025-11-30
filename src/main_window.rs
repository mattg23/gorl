use std::cell::RefCell;
use std::fs::File;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::RwLock;

use crate::highlighter::Highlighter;
use crate::lineview::LineBasedFileView;

use crate::search::SearchWindow;
use fltk::draw;
use fltk::{
    app,
    enums::Event,
    frame::Frame,
    prelude::*,
    valuator::Scrollbar,
    window::{self, DoubleWindow},
};
use skia_safe::{
    Font, Paint,
    textlayout::{FontCollection, ParagraphBuilder, ParagraphStyle, TextStyle},
};

use log::{debug, error, info};

use crate::SETTINGS;

use crate::common::{GorlMsg, WindowId, next_window_id};

#[derive(Copy, Clone, Debug)]
pub(crate) enum MwMessage {
    JumpTo(u64),
}
use skia_safe::{AlphaType, ColorType, ImageInfo};

pub struct SkiaView {
    pub buf: Vec<u8>,
    pub stride: usize,
    pub font_collection: FontCollection,
    pub width: i32,
    pub height: i32,
}

pub(crate) struct GorlLogWindow {
    view: Rc<RwLock<Option<LineBasedFileView<File>>>>,
    search_window: Option<SearchWindow>,
    outbox: app::Sender<GorlMsg>,
    id: WindowId,
    highlighter: Highlighter, //transmitter: Sender<MwMessage>,
    window: Option<DoubleWindow>,
    frame: Frame,
    right_scroll: Scrollbar,
    skia: Rc<RefCell<SkiaView>>,
}

impl SkiaView {
    pub fn new(w: i32, h: i32) -> Self {
        let w_usize = w as usize;
        let h_usize = h as usize;

        let stride = w_usize * 4;
        let size = stride * h_usize;

        let buf = vec![0u8; size];

        let settings = SETTINGS.read().expect("SETTINGS");

        let font_collection = {
            // We need a system font manager to be able to load typefaces.
            let font_mgr = skia_safe::FontMgr::new();

            let mut font_collection = FontCollection::new();
            font_collection.set_default_font_manager(font_mgr, settings.font.name.as_str());
            font_collection.enable_font_fallback();
            font_collection
        };
        Self {
            buf,
            stride,
            font_collection,
            width: w,
            height: h,
        }
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        if self.width == w && self.height == h {
            return;
        }

        self.width = w;
        self.height = h;
        self.stride = (w as usize) * 4;
        self.buf.resize(self.stride * (h as usize), 0);
    }

    pub fn max_visible_lines(&self) -> u64 {
        (self.height / SETTINGS.read().unwrap().font.size) as u64
    }

    pub fn draw(&mut self, text: &Vec<(u64, String)>, max_line_count: u64) {
        let info = ImageInfo::new(
            (self.width, self.height),
            ColorType::RGBA8888,
            AlphaType::Unpremul,
            None,
        );

        let mut surface =
            skia_safe::surfaces::wrap_pixels(&info, self.buf.as_mut(), self.stride, None).unwrap();

        let canvas = surface.canvas();
        canvas.clear(skia_safe::Color::from_argb(255, 30, 30, 30));

        let mut paint = Paint::default();
        paint.set_color(skia_safe::Color::from_argb(255, 230, 230, 230));
        let mut paint_ln = Paint::default();
        paint_ln.set_color(skia_safe::Color::from_argb(160, 230, 230, 230));
        let mut paint_ln_zeros = Paint::default();
        paint_ln_zeros.set_color(skia_safe::Color::from_argb(60, 230, 230, 230));

        let mut ln_bg = Paint::default();
        ln_bg.set_color(skia_safe::Color::from_argb(255, 10, 10, 10));

        let font = Font::new(
            self.font_collection.default_fallback().unwrap(),
            SETTINGS.read().unwrap().font.size as f32,
        );
        let f_size = font.size().floor() as usize;

        let max_str = format!("{max_line_count}");
        let char_width = max_str.len();
        let max_width = font.measure_str(max_str.as_str(), None).0.ceil();

        canvas.draw_rect(
            skia_safe::Rect {
                left: 0.0,
                right: max_width + 8.0,
                top: 0.0,
                bottom: self.height as f32,
            },
            &ln_bg,
        );

        for (i, (ln, line)) in text.iter().enumerate() {
            let ln_str = format!("{ln}");
            let cnt_zeros = char_width - ln_str.len();
            let mut ln_zeros_width = 0;
            if cnt_zeros > 0 {
                let ln_zeros = format!("{:0cnt_zeros$}", 0);
                ln_zeros_width = font.measure_str(ln_zeros.as_str(), None).0 as i32;
                canvas.draw_str(
                    ln_zeros,
                    (4, ((i + 1) * f_size) as i32),
                    &font,
                    &paint_ln_zeros,
                );
            }
            canvas.draw_str(
                ln_str,
                (4 + ln_zeros_width, ((i + 1) * f_size) as i32),
                &font,
                &paint_ln,
            );
            // canvas.draw_str(
            //     &line.1.as_str(),
            //     (max_width as i32 + 16, ((i + 1) * f_size) as i32),
            //     &font,
            //     &paint,
            // );

            let mut paragraph_style = ParagraphStyle::new();
            paragraph_style.set_max_lines(1);
            paragraph_style.set_text_align(skia_safe::textlayout::TextAlign::Left);
            paragraph_style.set_text_direction(skia_safe::textlayout::TextDirection::LTR);
            let mut paragraph_builder =
                ParagraphBuilder::new(&paragraph_style, &self.font_collection);
            let mut ts = TextStyle::new();
            ts.set_font_size(f_size as f32);
            ts.set_baseline_shift(1.0);
            ts.set_foreground_paint(&paint);
            paragraph_builder.push_style(&ts);
            paragraph_builder.add_text(&line.as_str());
            let mut paragraph = paragraph_builder.build();
            paragraph.layout(f32::MAX);
            paragraph.paint(&canvas, (max_width as i32 + 16, ((i) * f_size) as i32));
        }
    }
}

const SBWIDTH: i32 = 25;

impl GorlLogWindow {
    pub fn new() -> Self {
        info!("Creating Gorl Log Window. Settings = {:?}", SETTINGS.read());
        let id = next_window_id();

        let (s, receiver) = app::channel();

        let settings_lck = SETTINGS.read().unwrap();
        let highlight_settings = settings_lck.default_highlights.as_ref();

        let highlighter = Highlighter::new(highlight_settings.map_or(vec![], |a| a.clone()));

        let mut win = window::Window::default()
            .with_size(1200, 800)
            .with_label("GORL 🪵🪟");

        let mut frame = Frame::default().size_of(&win);

        let skia = Rc::new(RefCell::new(SkiaView::new(1200 - SBWIDTH, 800)));

        let skia_ = skia.clone();

        frame.handle({
            let outbox = s.clone();
            let id = id;
            let mut dnd = false;
            let sk = skia_.clone();

            move |f, ev| match ev {
                Event::DndEnter => {
                    dnd = true;
                    true
                }
                Event::DndDrag => true,    // accept drag
                Event::DndRelease => true, // accept release
                Event::Paste => {
                    if dnd {
                        let raw = app::event_text();

                        for uri in raw.split_whitespace() {
                            let uri = uri.trim();
                            let uri = uri.strip_prefix("file://").unwrap_or(uri);
                            let pb = PathBuf::from(uri);

                            if pb.exists() {
                                outbox.send(GorlMsg::OpenFileIn(id, pb));
                            }
                        }

                        dnd = false;
                    }
                    true
                }
                Event::Resize => {
                    let w = f.width();
                    let h = f.height();
                    sk.borrow_mut().resize(w, h);
                    outbox.send(GorlMsg::Refresh(id));
                    false
                }
                Event::DndLeave => {
                    dnd = false;
                    true
                }
                Event::NoEvent => false,
                _ => {
                    debug!("{:?}", ev);
                    false
                }
            }
        });

        let mut sb = fltk::valuator::Scrollbar::new(1200 - SBWIDTH, 0, SBWIDTH, 800, None);

        sb.handle({
            let outbox = s.clone();
            move |b, ev| {
                //dbg!(ev);
                match ev {
                    Event::Drag | Event::Released | Event::MouseWheel => {
                        outbox.send(GorlMsg::Refresh(id));
                    }
                    _ => (),
                };
                false
            }
        });

        win.end();
        win.resizable(&frame);
        win.resizable(&sb);
        win.make_resizable(true);
        win.show();

        let mut self_ = Self {
            view: Rc::new(RwLock::new(None)),
            search_window: None,
            outbox: s,
            highlighter,
            window: Some(win),
            id,
            frame,
            skia: skia,
            right_scroll: sb,
        };

        self_
    }

    fn draw_text(&mut self) {
        if let Ok(mut lock) = self.view.write() {
            if let Some(view) = lock.as_mut() {
                let current = (self.right_scroll.value() as u64).max(1);

                let max = view.line_count();
                let visible = self.skia.borrow().max_visible_lines();
                let take = visible.min(max);
                let mut lines = Vec::with_capacity(take as usize);
                for i in (current - 1)..(current + take).min(max) {
                    if let Ok(line) = view.get_line(i) {
                        lines.push(((i + 1) as u64, line));
                    }
                }

                self.skia.borrow_mut().draw(&lines, max);
                self.right_scroll
                    .scroll_value(current as i32, visible as i32, 1, max as i32);
            }
        }
    }

    fn redraw_frame(&mut self) {
        self.draw_text();
        draw::draw_rgba(&mut self.frame, &self.skia.borrow().buf)
            .expect("redraw_frame:: could not draw into frame");
        self.frame.redraw();
        self.right_scroll.redraw();
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

        debug!("process_message: {msg:?}");

        match msg {
            GorlMsg::OpenFileIn(w, path) if *w == id => match self.open_file(path) {
                Ok(view) => {
                    *self.view.write().unwrap() = Some(view);

                    self.redraw_frame();
                    self.window
                        .as_mut()
                        .expect("dropped a file into an hidden window?")
                        .set_label(format!("GORL 🪵🪟 - {path:?}").as_str());
                    self.redraw_frame();
                }
                Err(e) => {
                    error!("could not open {path:?}. ERR={e:?}");
                }
            },
            GorlMsg::Refresh(w) if *w == id => self.redraw_frame(),
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
