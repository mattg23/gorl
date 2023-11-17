use std::rc::Rc;
use std::sync::RwLock;
use grep::regex::RegexMatcher;
use grep::searcher::{BinaryDetection, SearcherBuilder};
use grep::searcher::sinks::UTF8;
use log::{debug, error, info};
use winsafe::{prelude::*, gui, co, HFONT, SIZE};
use winsafe::co::{CHARSET, CLIP, COLOR, ES, FW, LVS, LVS_EX, OUT_PRECIS, PITCH, QUALITY, WS};
use winsafe::gui::{Brush, Horz, ListViewOpts, Vert};
use winsafe::msg::wm::SetFont;
use crate::SETTINGS;

fn search_in_file(query: &str, path: &str) -> anyhow::Result<Vec<(u64, String)>> {

    let mut res = vec![];

    let matcher = RegexMatcher::new_line_matcher(query)?;
    let mut searcher =
        SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(b'\x00'))
            .line_number(true)
            .build();

    searcher.search_path(
        matcher,
        path,
        UTF8(|lnum, line| {
            res.push((lnum, line.to_owned()));
            Ok(true)
        })
    )?;

    Ok(res)
}

#[derive(Clone)]
pub struct SearchWindow {
    wnd: gui::WindowModeless,
    search_query_txt_box: gui::Edit,
    search_results: gui::ListView,
    search_button: gui::Button,
    current_file: Rc<RwLock<Option<String>>>,
}

impl SearchWindow
{
    pub fn new(parent: &impl GuiParent) -> Self {

        let wnd = gui::WindowModeless::new(
                                        parent,
                                        gui::WindowModelessOpts {
                                            class_bg_brush: Brush::Color(COLOR::BACKGROUND),
                                            title: "GORL - Search".to_string(),
                                            style: gui::WindowMainOpts::default().style |
                                                co::WS::MINIMIZEBOX | co::WS::MAXIMIZEBOX | co::WS::SIZEBOX | WS::POPUPWINDOW,
                                            size: (600, 350),
                                            ..Default::default() // leave all other options as default
                                        },
        );

        let search_button = gui::Button::new(&wnd, gui::ButtonOpts{
            height: 24,
            width: 150,
            text: " 🔍 Search".to_owned(),
            position: (420, 10),
            resize_behavior: (Horz::Repos, Vert::None),
            ..Default::default()
        });

        let search_query_txt_box = gui::Edit::new(&wnd, gui::EditOpts{
           text: "".to_string(),
            position: (10, 10),
            width: 400,
            height: 24,
            edit_style: ES::LEFT,
            resize_behavior: (Horz::Resize, Vert::None),
            ..Default::default()
        });

        let search_results = gui::ListView::new(&wnd, ListViewOpts {
            position: (10, 44),
            size: (560, 256),
            columns: vec![("Line".to_string(), 128), ("Text".to_string(), 3200)],
            resize_behavior: (Horz::Resize, Vert::Resize),
            list_view_ex_style: LVS_EX::DOUBLEBUFFER | LVS_EX::FULLROWSELECT,
            list_view_style: LVS::REPORT | LVS::NOLABELWRAP,
            ..Default::default()
        });

        let mut new_self = Self {
            wnd, search_query_txt_box, search_results, search_button,
            current_file: Rc::new(RwLock::new(None)),
        };

        new_self.events(); // attach our events
        new_self
    }

    pub fn set_file(&self, new_path: &str) {
        *self.current_file.write().unwrap() = Some(new_path.to_owned());
        info!("SEARCHWINDOW: set file to {new_path}");
    }

    fn events(&mut self) {
        self.wnd.on().wm_create({
            let myself = self.clone();
            move |_msg| {
                info!("SEARCH WINDOW: WM_CREATE");
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

                    myself.search_query_txt_box.set_font(&font);

                    myself.search_results.hwnd().SendMessage(SetFont {
                        hfont: font.leak(),
                        redraw: true,
                    }.as_generic_wm());
                }
                Ok(0)
            }
        });

        self.search_results.on().nm_dbl_clk({
            let myself = self.clone();
            move |msg| {
                let index = msg.iItem;
                let lnum_str = myself.search_results.items().get(index as u32).text(0);

                if let Ok(num) = lnum_str.as_str().parse::<u64>() {
                    debug!("SEARCH WINDOW: USER DOUBLE CLICKED ON ITEM {index} => parse to line {num}");

                }

                Ok(())
            }
        });

        self.search_button.on().bn_clicked( {
            let myself = self.clone();
            move || {
                info!("SEARCH WINDOW: SEARCH CLICKED");
                if let Ok(lock_res) = myself.current_file.read() {
                    if let Some(file) = lock_res.as_ref() {
                        let query = myself.search_query_txt_box.text();
                         match  search_in_file(query.as_str(), file.as_str()) {
                             Ok(search_results) => {
                                 myself.search_results.items().delete_all();
                                 let items = myself.search_results.items();
                                 for (lnum, line) in &search_results {
                                     items.add(&[
                                         format!("{lnum}").as_str(),
                                         line.as_str()
                                     ], None);
                                 }
                                 info!("SEARCH WINDOW: SEARCH EXECUTED. #RES={}", search_results.len());
                             }
                             Err(err) => {
                                 error!("SEARCH WINDOW: ERROR DURING SEARCH: {err}");
                             }
                         }
                    }
                }
                Ok(())
            }
        })
    }
}