use std::collections::Bound;
use std::fs::{File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::ops::{Deref, RangeBounds};
use std::rc::Rc;
use std::sync::RwLock;

use winsafe::{prelude::*, gui, co, WString, HFONT, SIZE};
use winsafe::co::{CHARSET, CLIP, FW, LVS, LVS_EX, OUT_PRECIS, PITCH, QUALITY};
use winsafe::guard::DeleteObjectGuard;
use winsafe::gui::{Horz, ListViewOpts, Vert};
use winsafe::msg::wm::SetFont;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let file_path = "D:\\Dump\\opc_log\\ops_logs_23Mar23\\Service.20230322.00.log";

    let view = LineBasedFileView::new(file_path.to_string())?;

    let my = GorlMainWindow::new(Rc::new(RwLock::new(view))); // instantiate our main window

    if let Err(e) = my.wnd.run_main(None) { // ... and run it
        eprintln!("{}", e);
    }

    //view.cache_lines(5000..=6000);

    Ok(())
}

#[derive(Clone)]
pub struct GorlMainWindow {
    wnd: gui::WindowMain,
    // responsible for managing the window
    list_view: gui::ListView,
    // a button
    view: Rc<RwLock<LineBasedFileView>>,
}

impl GorlMainWindow {
    pub fn new(view: Rc<RwLock<LineBasedFileView>>) -> Self {
        let wnd = gui::WindowMain::new( // instantiate the window manager
                                        gui::WindowMainOpts {
                                            title: "GORL".to_owned(),
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

        // for (i, l) in lines.iter().enumerate() {
        //     //format!("{i}"), l.to_string()
        //     list_view.items().add(&[
        //         "1",
        //         "2"
        //     ], None);
        // }


        let mut new_self = Self { wnd, list_view, view };
        new_self.events(); // attach our events
        new_self
    }

    fn open_file(&self, path: &String) -> anyhow::Result<LineBasedFileView> {
        let view = LineBasedFileView::new(path.clone())?;
        Ok(view)
    }

    fn events(&mut self) {
        //let wnd = self.wnd.clone(); // clone so it can be passed into the closure

        self.wnd.on().wm_create({
            let myself = self.clone();
            move |msg| {
                // for (i, l) in myself.lines.iter().enumerate() {
                //     //f
                //     let item  = myself.list_view.items().add(&[
                //         format!("{i}"), l.to_string()
                //     ], None);
                // }

                myself.list_view.items().set_count((myself.view.read().unwrap().line_count() - 1) as u32, None);
                myself.wnd.hwnd().DragAcceptFiles(true);

                //myself.list_view.hwnd().

                let mut font = HFONT::CreateFont(
                    SIZE::new(8, 0),
                    0,
                    0,
                    FW::MEDIUM,
                    false,
                    false,
                    false,
                    CHARSET::DEFAULT,
                    OUT_PRECIS::DEFAULT,
                    CLIP::DEFAULT_PRECIS,
                    QUALITY::DEFAULT,
                    PITCH::FIXED,
                    "Comic Sans MS"
                )?;

                myself.list_view.hwnd().SendMessage(SetFont {
                    hfont: font.leak(),
                    redraw: true
                }.as_generic_wm());


                Ok(0)
            }
        });

        self.wnd.on().wm_drop_files({
            let mut myself = self.clone();
            move |mut msg| {
                if let Ok(itr) = msg.hdrop.DragQueryFile() {
                    for f in itr {
                        println!("{:?}", f);
                        if let Ok(f) = f {
                            match (myself).open_file(&f) {
                                Ok(view) => {
                                    {
                                        *myself.view.write().unwrap() = view;
                                    }
                                    myself.list_view.items().set_count((myself.view.read().unwrap().line_count() - 1) as u32, None);
                                    myself.wnd.set_text(f.as_str());
                                    println!("set {f}. lines = {}", myself.view.read().unwrap().line_count());
                                }
                                Err(e) => {
                                    println!("could not open {f}. ERR={:?}", e)
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
                if info.item.mask.has(co::LVIF::TEXT) { // is this a text request?
                    //println!("{} {}", info.item.iItem, info.item.cColumns);
                    let index = info.item.iItem as usize;
                    if info.item.cColumns == 0 {
                        let (ptr, cch) = info.item.raw_pszText(); // retrieve raw pointer
                        let out_slice = unsafe { std::slice::from_raw_parts_mut(ptr, cch as _) };
                        out_slice.iter_mut()
                            .zip(WString::from_str(format!("{}", index + 1)).as_slice())
                            .for_each(|(dest, src)| *dest = *src); // copy from our string to their buffer
                    } else {
                        let mut view_ref = myself.view.write().unwrap();
                        let str_ref = view_ref.get_line(index as u64); // string for the requested item
                        let (ptr, cch) = info.item.raw_pszText(); // retrieve raw pointer
                        let out_slice = unsafe { std::slice::from_raw_parts_mut(ptr, cch as _) };
                        out_slice.iter_mut()
                            .zip(WString::from_str(str_ref.unwrap_or_else(|e| e)).as_slice())
                            .for_each(|(dest, src)| *dest = *src); // copy from our string to their buffer
                    }
                }

                Ok(())
            }
        });
        self.list_view.on().lvn_od_cache_hint(|f| {
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
    file_path: String,
    reader: BufReader<File>,
    lines: Vec<u64>,
    line_cache: Vec<String>,
    last_bounds: Option<LastBound>,
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

        // let mut buf: [u8; 1024] = [0x0; 1024];
        // while let Ok(read) = reader.read(&mut buf) {
        //     if read == 0 {
        //         break;
        //     }
        //
        //     reader.stream_position()
        //
        //     for (i, c) in buf[0..read].iter().enumerate() {
        //         if *c == b'\n' {
        //             lines.push(lines.last().unwrap_or(&0) + (i + 1) as u64)
        //         }
        //     }
        // }

        Ok(Self {
            lines,
            file_path,
            reader,
            line_cache: vec![],
            last_bounds: None,
        })
    }

    pub fn line_count(&self) -> u64 {
        self.lines.len() as u64
    }

    pub fn get_line(&mut self, index: u64) -> Result<String, String> {
        if let Some(last_bounds) = &self.last_bounds {
            if last_bounds.left <= index && index < last_bounds.right {
                if let Some(line) = self.line_cache.get((index - last_bounds.left) as usize) {
                    return Ok(line.clone());
                } else {
                    return Err(format!("ERROR READING LINE {index} with ERR: NOT FOUND"));
                }
            }
        }

        const DEF_CACHE_RANGE: u64 = 500;

        let left = if index > DEF_CACHE_RANGE {
            index - DEF_CACHE_RANGE
        } else {
            0
        };
        match self.cache_lines(left..=u64::min(index + DEF_CACHE_RANGE, self.lines.len() as u64)) {
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
        let mut buf = Vec::with_capacity(buf_length);
        buf.resize(buf_length, 0);

        let bytes_read = self.reader.read_exact(buf.as_mut_slice())?;

        let res = BufReader::new(buf.as_slice());
        self.line_cache = res.lines().map(|l| l.unwrap()).collect();

        println!("LEFTOFFSET = {left_offset} || RIGHTOFFSET = {right_offset} || R.START = {:?} || R.END = {:?} || SELF.LASTBOUNDS = {:?} || CACHELEN = {}", r.start_bound(), r.end_bound(), &self.last_bounds, self.line_cache.len());

        Ok(())
    }
}


