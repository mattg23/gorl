use crate::{SETTINGS};
use grep::regex::RegexMatcherBuilder;
use grep::searcher::sinks::UTF8;
use grep::searcher::{BinaryDetection, SearcherBuilder};
use log::{debug, error, info};
use std::rc::Rc;
use std::sync::{RwLock};
use flume::{Sender};

use winsafe::co::{BS, CHARSET, CLIP, COLOR, ES, FW, LVS, LVS_EX, OUT_PRECIS, PITCH, QUALITY, VK, WS};
use winsafe::gui::{Brush, Horz, ListViewOpts, Vert};
use winsafe::msg::wm::SetFont;
use winsafe::{co, gui, prelude::*, HFONT, SIZE, WString};

use crate::main_window::MwMessage;

fn search_in_file(query: &str, path: &str) -> anyhow::Result<Vec<(u64, String)>> {
    let mut res = vec![];

    let matcher = RegexMatcherBuilder::default()
        .case_insensitive(true)
        .line_terminator(Some(b'\n'))
        .build(query)?;

    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .line_number(true)
        .build();

    searcher.search_path(
        matcher,
        path,
        UTF8(|lnum, line| {
            res.push((lnum, line.to_owned()));
            Ok(true)
        }),
    )?;

    Ok(res)
}

type SearchResults = Rc<RwLock<Option<Vec<(u64, String)>>>>;

#[derive(Clone)]
pub(crate) struct SearchWindow {
    wnd: gui::WindowModeless,
    search_query_txt_box: gui::Edit,
    search_results_list: gui::ListView,
    search_button: gui::Button,
    current_file: Rc<RwLock<Option<String>>>,
    transmitter: Sender<MwMessage>,
    current_search_results: SearchResults,
}

impl SearchWindow {
    pub fn new(parent: &impl GuiParent, transmitter: Sender<MwMessage>) -> Self {
        let wnd = gui::WindowModeless::new(
            parent,
            gui::WindowModelessOpts {
                class_bg_brush: Brush::Color(COLOR::BACKGROUND),
                title: "GORL - Search".to_string(),
                style: gui::WindowMainOpts::default().style
                    | WS::MINIMIZEBOX
                    | WS::MAXIMIZEBOX
                    | WS::SIZEBOX
                    | WS::POPUPWINDOW,
                size: (600, 350),
                ..Default::default() // leave all other options as default
            },
        );

        let search_button = gui::Button::new(
            &wnd,
            gui::ButtonOpts {
                height: 24,
                width: 150,
                text: " 🔍 Search".to_owned(),
                position: (420, 10),
                button_style: BS::DEFPUSHBUTTON | BS::PUSHBUTTON,
                resize_behavior: (Horz::Repos, Vert::None),
                ..Default::default()
            },
        );

        let search_query_txt_box = gui::Edit::new(
            &wnd,
            gui::EditOpts {
                text: "".to_string(),
                position: (10, 10),
                width: 400,
                height: 24,
                edit_style: ES::LEFT | ES::NOHIDESEL | ES::AUTOHSCROLL,
                resize_behavior: (Horz::Resize, Vert::None),
                ..Default::default()
            },
        );

        let search_results = gui::ListView::new(
            &wnd,
            ListViewOpts {
                position: (10, 44),
                size: (560, 256),
                columns: vec![("Line".to_string(), 128), ("Text".to_string(), 3200)],
                resize_behavior: (Horz::Resize, Vert::Resize),
                list_view_ex_style: LVS_EX::DOUBLEBUFFER | LVS_EX::FULLROWSELECT,
                list_view_style: LVS::REPORT | LVS::NOLABELWRAP | LVS::OWNERDATA,
                ..Default::default()
            },
        );

        let mut new_self = Self {
            wnd,
            search_query_txt_box,
            search_results_list: search_results,
            search_button,
            current_file: Rc::new(RwLock::new(None)),
            transmitter,
            current_search_results: Rc::new(RwLock::new(None)),
        };

        new_self.events(); // attach our events
        new_self
    }

    pub fn set_file(&self, new_path: &str) {
        *self.current_file.write().unwrap() = Some(new_path.to_owned());
        info!("SEARCHWINDOW: set file to {new_path}");
    }

    extern "system" fn handle_edit_text_box(h_wnd: winsafe::HWND, u_msg: co::WM, w_param: usize, l_param: isize, _u_id_subclass: usize, dw_ref_data: usize) -> isize {

        if u_msg == co::WM::KEYUP {
            unsafe {
                if VK::from_raw(w_param as u16) == VK::RETURN {
                    debug!("handle_edit_text_box::SubClassProcedure  {}, w_param={}, lParama={}",u_msg, VK::RETURN, l_param);
                    let ptr = dw_ref_data as *const Self;
                    (*ptr).search_button.trigger_click();
                    (*ptr).search_query_txt_box.focus();
                }
            }

        }
        let wm_any = winsafe::msg::WndMsg::new(u_msg, w_param, l_param);
        h_wnd.DefSubclassProc(wm_any)
    }

    extern "system" fn subclass_search_result_list_view(h_wnd: winsafe::HWND, u_msg: co::WM, w_param: usize, l_param: isize, _u_id_subclass: usize, dw_ref_data: usize) -> isize {
        if u_msg == co::WM::KEYDOWN {
            unsafe {
                if VK::from_raw(w_param as u16) == VK::CHAR_C && winsafe::GetAsyncKeyState(VK::CONTROL) {
                    let is_shift_down = winsafe::GetAsyncKeyState(VK::SHIFT);

                    let ptr = dw_ref_data as *const Self;


                    let sel_count = (*ptr).search_results_list.items().selected_count();
                    if 0 < sel_count && sel_count <= SETTINGS.read().unwrap().max_nb_of_lines_to_copy {
                        let mut str_to_cpy = String::new();

                        for sel_item in (*ptr).search_results_list.items().iter_selected() {
                            if is_shift_down {
                                str_to_cpy.push_str(sel_item.text(0).as_str());
                                str_to_cpy.push_str(" | ");
                            }
                            str_to_cpy.push_str(sel_item.text(1).as_str());
                        }

                        match crate::utils::copy_text_to_clipboard(&h_wnd, str_to_cpy.as_str()) {
                            Ok(_) => { info!("subclass_list_view::SubClassProcedure: clipboard data has been set!") }
                            Err(e) => { error!("subclass_list_view::SubClassProcedure: could not set clipboard data: {e}") }
                        }
                    }
                }

                debug!("subclass_list_view::SubClassProcedure {}, w_param={}, lParama={}",u_msg, w_param, l_param);
            }
        }
        let wm_any = winsafe::msg::WndMsg::new(u_msg, w_param, l_param);
        h_wnd.DefSubclassProc(wm_any)
    }

    fn events(&mut self) {
        self.wnd.on().wm_create({
            let myself = self.clone();
            move |_msg| {
                info!("SEARCH WINDOW: WM_CREATE");
                let _ = crate::utils::try_set_dark_mode(myself.wnd.hwnd());
                if let Ok(settings) = SETTINGS.read() {
                    let mut font = HFONT::CreateFont(
                        SIZE::new(0,settings.font.size),
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

                    myself.search_results_list.hwnd().SendMessage(
                        SetFont {
                            hfont: font.leak(),
                            redraw: true,
                        }
                            .as_generic_wm(),
                    );


                    unsafe { let _ = myself.search_query_txt_box.hwnd().SetWindowSubclass(Self::handle_edit_text_box, 0, &myself as *const _ as _); }
                    unsafe { let _ = myself.search_results_list.hwnd().SetWindowSubclass(Self::subclass_search_result_list_view, 0, &myself as *const _ as _); }

                }
                Ok(0)
            }
        });


        self.search_query_txt_box.on().en_update({
            let myself = self.clone();
            move ||{
                let text = myself.search_query_txt_box.text();
                const ASCII_DELETE : char = '\u{7f}';
                if text.ends_with(ASCII_DELETE) { // ends in ASCII 127 == DELETE character

                    let (i,_) = text.char_indices().rfind(|(_,c)| c.ne(&ASCII_DELETE) && c.is_whitespace()).unwrap_or((0, 's'));

                    let next = &text[0..i];
                    myself.search_query_txt_box.set_text(next);
                    myself.search_query_txt_box.set_selection(i as i32, i as i32);
                }
                Ok(())
            }
        });

        self.search_results_list.on().nm_dbl_clk({
            let myself = self.clone();
            move |msg| {
                let index = msg.iItem;
                let lnum_str = myself.search_results_list.items().get(index as u32).text(0);

                if let Ok(num) = lnum_str.as_str().parse::<u64>() {
                    debug!(
                        "SEARCH WINDOW: USER DOUBLE CLICKED ON ITEM {index} => parse to line {num}"
                    );

                    myself.transmitter.send(MwMessage::JumpTo(num))?;
                }

                Ok(())
            }
        });

        self.search_results_list.on().lvn_get_disp_info({
            let myself = self.clone();
            move |info| {
                if myself.current_search_results.read().is_ok_and(|o| o.is_none()) {
                    return Ok(());
                }

                if info.item.mask.has(co::LVIF::TEXT) {
                    let index = info.item.iItem as usize;
                    let line_set = match myself.current_search_results.read() {
                        Ok(guard) => {
                            if guard.is_some() {
                                let results = guard.as_ref().unwrap();
                                if let Some((lnum, line)) = results.get(index) {
                                    let text_to_set = if info.item.iSubItem == 0 {
                                        // first col
                                        WString::from_str(format!("{lnum}"))
                                    } else {
                                        WString::from_str(line)
                                    };

                                    let (ptr, cch) = info.item.raw_pszText(); // retrieve raw pointer
                                    let out_slice =
                                        unsafe { std::slice::from_raw_parts_mut(ptr, cch as _) };
                                    out_slice
                                        .iter_mut()
                                        .zip(text_to_set.as_slice())
                                        .for_each(|(dest, src)| *dest = *src); // copy from our string to their buffer
                                    Ok(())
                                } else {
                                    Err(format!("Line not found with index {index} "))
                                }
                            } else {
                                Err("No search results available".to_string())
                            }
                        }
                        Err(error) => {
                            Err(format!("{error}"))
                        }
                    };

                    if line_set.is_err() {
                        error!("SeachWindow: ERROR SETTING ITEM TEXT {index} {:?}", line_set.unwrap_err());
                    }
                }

                Ok(())
            }
        });

        self.search_button.on().bn_clicked({
            let myself = self.clone();
            move || {
                info!("SEARCH WINDOW: SEARCH CLICKED");
                if let Ok(lock_res) = myself.current_file.read() {
                    if let Some(file) = lock_res.as_ref() {
                        let query = myself.search_query_txt_box.text();
                        match search_in_file(query.as_str(), file.as_str()) {
                            Ok(search_results) => {
                                let len = search_results.len();
                                if let Ok(mut guard) = myself.current_search_results.write() {
                                    *guard = Some(search_results);

                                    myself.wnd.set_text(
                                        format!(
                                            "GORL - Search - #RES={} [{}]",
                                            len,
                                            myself.current_file.read().unwrap().as_ref().unwrap()
                                        )
                                            .as_str(),
                                    );

                                    info!("SEARCH WINDOW: SEARCH EXECUTED. #RES={}",len);
                                    myself.search_results_list.items().delete_all();
                                    myself.search_results_list.items().set_count(len as u32, None);
                                } else {
                                    error!("COULD NOT LOCK SearchWindow.current_search_results")
                                }
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
