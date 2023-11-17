mod settings;
mod search;

use std::collections::Bound;
use std::fs::{File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::ops::{RangeBounds};
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::RwLock;
use lazy_static::lazy_static;
use log::{debug, error, info};

use winsafe::{prelude::*, gui, co, WString, HFONT, SIZE};
use winsafe::co::{CHARSET, CLIP, FW, LVS, LVS_EX, OUT_PRECIS, PITCH, QUALITY};
use winsafe::gui::{Horz, ListViewOpts, Vert};
use winsafe::msg::wm::SetFont;
use crate::search::SearchWindow;

lazy_static! {
    static ref SETTINGS: RwLock<settings::Settings> = RwLock::new(settings::Settings::new());
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let my = GorlMainWindow::new(Rc::new(RwLock::new(None))); // instantiate our main window

    if let Err(e) = my.wnd.run_main(None) {
        // ... and run it
        error!("{}", e);
    }

    Ok(())
}

#[derive(Clone)]
pub struct GorlMainWindow {
    wnd: gui::WindowMain,
    list_view: gui::ListView,
    view: Rc<RwLock<Option<LineBasedFileView>>>,
    search_window: SearchWindow,
}

impl GorlMainWindow {
    pub fn new(view: Rc<RwLock<Option<LineBasedFileView>>>) -> Self {
        let wnd = gui::WindowMain::new( // instantiate the window manager
                                        gui::WindowMainOpts {
                                            title: "GORL - Drag text file into view to start...".to_owned(),
                                            size: (900, 600),
                                            style: gui::WindowMainOpts::default().style |
                                                co::WS::MINIMIZEBOX | co::WS::MAXIMIZEBOX | co::WS::SIZEBOX,
                                            ..Default::default() // leave all other options as default
                                        },
        );


        let list_view = gui::ListView::new(&wnd, ListViewOpts {
            position: (10, 10),
            size: (880, 580),
            columns: vec![("L".to_string(), 128), ("Text".to_string(), 3200)],
            resize_behavior: (Horz::Resize, Vert::Resize),
            list_view_ex_style: LVS_EX::DOUBLEBUFFER | LVS_EX::FULLROWSELECT,
            list_view_style: LVS::REPORT | LVS::OWNERDATA | LVS::NOLABELWRAP,
            ..Default::default()
        });


        let search_window = SearchWindow::new(&wnd);
        let mut new_self = Self { wnd, list_view, view, search_window };
        new_self.events(); // attach our events
        new_self
    }

    fn open_file(&self, path: &str) -> anyhow::Result<LineBasedFileView> {
        let view = LineBasedFileView::new(path.to_owned())?;
        Ok(view)
    }

    fn jump_to(&self, line: u64) {
        debug!("MAIN WINDOW: RECEIVED SEARCH RESULT SELEDTED {line}");
        let item = self.list_view.items().get(line as u32);
        item.select(true);
        item.ensure_visible();
        item.focus();
    }

    fn events(&mut self) {

        self.wnd.on().wm_create({
            let myself = self.clone();
            move |_msg| {
                info!("WM_CREATE");
                myself.wnd.hwnd().DragAcceptFiles(true);

                if let Ok(settings) = SETTINGS.read() {
                    let mut font = HFONT::CreateFont(
                        SIZE::new(settings.font.size, 0),
                        0,
                        0,
                        FW::MEDIUM,
                        settings.font.italic,
                        false,
                        false,
                        CHARSET::DEFAULT,
                        OUT_PRECIS::DEFAULT,
                        CLIP::DEFAULT_PRECIS,
                        QUALITY::DEFAULT,
                        PITCH::FIXED,
                        settings.font.name.as_str(),
                    )?;

                    myself.list_view.hwnd().SendMessage(SetFont {
                        hfont: font.leak(),
                        redraw: true,
                    }.as_generic_wm());
                }
                Ok(0)
            }
        });

        self.wnd.on().wm_drop_files({
            let myself = self.clone();
            move |mut msg| {
                if let Ok(itr) = msg.hdrop.DragQueryFile() {
                    for f in itr {
                        info!("Dropped FILE={:?}", f);
                        if let Ok(f) = f {
                            match (myself).open_file(&f) {
                                Ok(view) => {
                                    {
                                        *myself.view.write().unwrap() = Some(view);
                                    }
                                    myself.list_view.items().set_count((myself.view.read().unwrap().as_ref().unwrap().line_count() - 1) as u32, None);
                                    myself.wnd.set_text(format!("GORL - {f}").as_str());
                                    info!("set {f}. lines = {}", myself.view.read().unwrap().as_ref().unwrap().line_count());
                                    myself.search_window.set_file(&f);
                                }
                                Err(e) => {
                                    error!("could not open {f}. ERR={:?}", e)
                                }
                            }
                        }
                    }
                }
                Ok(())
            }
        });

        self.list_view.on().lvn_get_disp_info({
            let myself = self.clone();
            move |info| {
                if myself.view.read().is_ok_and(|o| o.is_none()) {
                    return Ok(());
                }

                if info.item.mask.has(co::LVIF::TEXT) { // is this a text request?
                    //println!("iItem={}; iSubItem={}; cColumns={};", info.item.iItem, info.item.iSubItem,info.item.cColumns);
                    let index = info.item.iItem as usize;
                    if info.item.iSubItem == 0 {
                        let (ptr, cch) = info.item.raw_pszText(); // retrieve raw pointer
                        let out_slice = unsafe { std::slice::from_raw_parts_mut(ptr, cch as _) };
                        out_slice.iter_mut()
                            .zip(WString::from_str(format!("{}", index + 1)).as_slice())
                            .for_each(|(dest, src)| *dest = *src); // copy from our string to their buffer
                    } else {
                        let line_text =
                            if let Ok(mut lock_res) = myself.view.write() {
                                if let Some(view_ref) = lock_res.as_mut() {
                                    Ok(view_ref.get_line(index as u64))
                                } else {
                                    Err("Could not get lock view ref mutably INNER")
                                }
                            } else {
                                Err("Could not get lock view ref mutably OUTER")
                            };

                        match line_text {
                            Ok(Ok(text)) => {
                                let (ptr, cch) = info.item.raw_pszText(); // retrieve raw pointer
                                let out_slice = unsafe { std::slice::from_raw_parts_mut(ptr, cch as _) };
                                out_slice.iter_mut()
                                    .zip(WString::from_str(text.as_str()).as_slice())
                                    .for_each(|(dest, src)| *dest = *src); // copy from our string to their buffer
                            }
                            r => error!("ERROR getting line: {:?}", r)
                        };
                    }
                }

                Ok(())
            }
        });
        self.list_view.on().lvn_od_cache_hint(|_f| {
            //println!("lvn_od_cache_hint from {} to {}", f.iFrom, f.iTo);
            Ok(())
        });
    }
}


#[derive(Debug, Copy, Clone)]
struct LastBound {
    pub left: u64,
    pub right: u64,
}

#[derive(Debug)]
pub struct LineBasedFileView {
    reader: BufReader<File>,
    lines: Vec<u64>,
    line_cache: Vec<String>,
    last_bounds: Option<LastBound>,
    def_cache_size: u64,
}

impl LineBasedFileView {
    pub fn new(file_path: String) -> anyhow::Result<Self> {
        let file = File::open(&file_path)?;
        let mut reader = BufReader::new(file);
        let mut lines: Vec<u64> = vec![0];

        let mut str_buf = String::new();
        while let Ok(bytes_read) = reader.read_line(&mut str_buf) {
            if bytes_read == 0 {
                break;
            }

            lines.push(reader.stream_position().unwrap());
        }

        let def_cache_size = if let Ok(settings) = SETTINGS.read() {
            settings.cache_size
        } else {
            settings::DEF_CACHE_RANGE
        };

        Ok(Self {
            lines,
            reader,
            line_cache: vec![],
            last_bounds: None,
            def_cache_size,
        })
    }

    pub fn line_count(&self) -> u64 {
        self.lines.len() as u64
    }

    pub fn get_line(&mut self, index: u64) -> Result<String, String> {
        if let Some(last_bounds) = &self.last_bounds {
            if last_bounds.left <= index && index < last_bounds.right {
                return if let Some(line) = self.line_cache.get((index - last_bounds.left) as usize) {
                    Ok(line.clone())
                } else {
                    Err(format!("ERROR READING LINE {index} with ERR: NOT FOUND"))
                };
            }
        }

        let def_cache_range = self.def_cache_size;

        let left = if index > def_cache_range {
            index - def_cache_range
        } else {
            0
        };
        match self.cache_lines(left..=u64::min(index + def_cache_range, self.lines.len() as u64)) {
            Ok(_) => { self.get_line(index) }
            Err(err) => {
                Err(format!("ERROR READING LINE {index} with ERR: {err}"))
            }
        }
    }

    fn cache_lines(&mut self, r: impl RangeBounds<u64>) -> anyhow::Result<()> {
        let left = match r.start_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => *i + 1,
            Bound::Unbounded => 0
        };

        let right = match r.end_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => *i - 1,
            Bound::Unbounded => (self.lines.len() - 1) as u64
        };

        let left_offset = *self.lines.get(left as usize).unwrap_or_else(|| self.lines.first().unwrap_or(&0));
        let right_offset = *self.lines.get(right as usize).unwrap_or_else(|| self.lines.last().unwrap_or(&0));

        self.last_bounds = Some(LastBound {
            left,
            right,
        });

        self.reader.seek(SeekFrom::Start(left_offset))?;

        let buf_length = (right_offset - left_offset) as usize;
        let mut buf = vec![0; buf_length];

        self.reader.read_exact(buf.as_mut_slice())?;

        let res = BufReader::new(buf.as_slice());
        self.line_cache = res.lines().map(|l| l.unwrap()).collect();

        debug!("LEFTOFFSET = {left_offset} || RIGHTOFFSET = {right_offset} || R.START = {:?} || R.END = {:?} || SELF.LASTBOUNDS = {:?} || CACHELEN = {}", r.start_bound(), r.end_bound(), &self.last_bounds, self.line_cache.len());

        Ok(())
    }
}


