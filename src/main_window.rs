use std::rc::Rc;
use std::sync::RwLock;

use crate::highlighter::Highlighter;
use crate::lineview::LineBasedFileView;
use winsafe::msg::wm::SetFont;
use winsafe::msg::WndMsg;

use crate::search::SearchWindow;
use flume::Receiver;
use log::{debug, error, info};
use winsafe::co::{CDDS, CHARSET, CLIP, FW, LVS, LVS_EX, OUT_PRECIS, PITCH, QUALITY, VK};
use winsafe::gui::{Horz, ListViewOpts, Vert};
use winsafe::{co, gui, prelude::*, WString, COLORREF, HFONT, HWND, SIZE};

use crate::SETTINGS;

#[derive(Copy, Clone, Debug)]
pub(crate) enum MwMessage {
    JumpTo(u64),
}

#[derive(Clone)]
pub(crate) struct GorlMainWindow {
    pub(crate) wnd: gui::WindowMain,
    list_view: gui::ListView,
    view: Rc<RwLock<Option<LineBasedFileView>>>,
    search_window: SearchWindow,
    inbox: Receiver<MwMessage>,
    highlighter: Highlighter, //transmitter: Sender<MwMessage>,
}

static CHECK_INBOX: co::WM = unsafe { co::WM::from_raw(0x1234) };

impl GorlMainWindow {
    pub fn new() -> Self {
        info!("Creating Main Window. Settings = {:?}", SETTINGS.read());

        let (transmitter, inbox) = flume::unbounded();

        let wnd = gui::WindowMain::new(
            // instantiate the window manager
            gui::WindowMainOpts {
                title: "GORL - Drag text file into view to start...".to_owned(),
                size: (900, 600),
                class_name: "GorlMainWindow".to_owned(),
                style: gui::WindowMainOpts::default().style
                    | co::WS::MINIMIZEBOX
                    | co::WS::MAXIMIZEBOX
                    | co::WS::SIZEBOX,
                ..Default::default() // leave all other options as default
            },
        );

        let list_view = gui::ListView::new(
            &wnd,
            ListViewOpts {
                position: (10, 10),
                size: (880, 580),
                columns: vec![("L".to_string(), 128), ("Text".to_string(), 3200)],
                resize_behavior: (Horz::Resize, Vert::Resize),
                list_view_ex_style: LVS_EX::DOUBLEBUFFER | LVS_EX::FULLROWSELECT,
                list_view_style: LVS::REPORT
                    | LVS::OWNERDATA
                    | LVS::NOLABELWRAP
                    | LVS::SHOWSELALWAYS,
                ..Default::default()
            },
        );

        let settings_lck = SETTINGS.read().unwrap();
        let highlight_settings = settings_lck.default_highlights.as_ref();

        let search_window = SearchWindow::new(&wnd, transmitter.clone());
        let mut new_self = Self {
            wnd: wnd.clone(),
            list_view,
            view: Rc::new(RwLock::new(None)),
            search_window,
            inbox: inbox.clone(),
            highlighter: Highlighter::new(highlight_settings.map_or( vec![] ,|a| a.clone())), //transmitter: transmitter.clone(),
        };

        let wnd_copy = wnd.clone();
        let rx_copy = inbox.clone();
        let tx_copy = transmitter.clone();

        wnd.spawn_new_thread(move || {
            debug!("Started Mainwindow CHECK_INBOX thread");

            while let Ok(msg) = rx_copy.recv() {
                debug!("Sending  CHECK_INBOX to MainWindow");
                // TODO: this is so hacky. Can we do it differently?
                match tx_copy.send(msg) {
                    Ok(_) => {
                        let check_inbox_msg = WndMsg::new(CHECK_INBOX, 0, 0);
                        let res = wnd_copy.hwnd().SendMessage(check_inbox_msg);
                        debug!("wnd_copy.hwnd().SendMessage returned = {res:?}")
                    }
                    Err(err) => {
                        error!("Sending  CHECK_INBOX to MainWindow: tx_copy.send(msg): {err}")
                    }
                };
            }

            Ok(())
        });

        new_self.events(); // attach our events
        new_self
    }

    fn open_file(&self, path: &str) -> anyhow::Result<LineBasedFileView> {
        let bf = std::time::SystemTime::now();
        let view = LineBasedFileView::new(path.to_owned())?;
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

        self.list_view.items().select_all(false);

        let item = self.list_view.items().get((line - 1) as u32);
        item.ensure_visible();
        item.focus();
        item.select(true);

        self.wnd.hwnd().SetForegroundWindow();
    }

    fn handle(&self, msg: MwMessage) {
        debug!("MainWindow Received: {msg:?}");

        match msg {
            MwMessage::JumpTo(line) => self.jump_to(line),
        }
    }

    extern "system" fn subclass_list_view(
        h_wnd: HWND,
        u_msg: co::WM,
        w_param: usize,
        l_param: isize,
        _u_id_subclass: usize,
        dw_ref_data: usize,
    ) -> isize {
        if u_msg == co::WM::KEYDOWN {
            unsafe {
                if VK::from_raw(w_param as u16) == VK::CHAR_C
                    && winsafe::GetAsyncKeyState(VK::CONTROL)
                {
                    let is_shift_down = winsafe::GetAsyncKeyState(VK::SHIFT);

                    let ptr = dw_ref_data as *const Self;

                    let sel_count = (*ptr).list_view.items().selected_count();
                    if 0 < sel_count
                        && sel_count <= SETTINGS.read().unwrap().max_nb_of_lines_to_copy
                    {
                        let mut str_to_cpy = String::new();

                        for sel_item in (*ptr).list_view.items().iter_selected() {
                            if is_shift_down {
                                str_to_cpy.push_str(sel_item.text(0).as_str());
                                str_to_cpy.push_str(" | ");
                            }
                            str_to_cpy.push_str(sel_item.text(1).as_str());
                            str_to_cpy.push_str("\r\n"); // Windows wants CRLF :(
                        }

                        match crate::utils::copy_text_to_clipboard(&h_wnd, str_to_cpy.as_str()) {
                            Ok(_) => {
                                info!("subclass_list_view::SubClassProcedure: clipboard data has been set!")
                            }
                            Err(e) => {
                                error!("subclass_list_view::SubClassProcedure: could not set clipboard data: {e}")
                            }
                        }
                    }
                }

                debug!(
                    "subclass_list_view::SubClassProcedure {}, w_param={}, lParama={}",
                    u_msg, w_param, l_param
                );
            }
        }
        let wm_any = WndMsg::new(u_msg, w_param, l_param);
        h_wnd.DefSubclassProc(wm_any)
    }

    fn events(&mut self) {
        self.wnd.on().wm(CHECK_INBOX, {
            let myself = self.clone();
            move |_| {
                debug!("MainWindow: RECEIVED CHECK_INBOX");

                match myself.inbox.try_recv() {
                    Ok(msg) => myself.handle(msg),
                    Err(err) => error!("MainWindow: ERROR getting MwMessage from inbox: {err}"),
                };
                Ok(Some(0))
            }
        });

        self.wnd.on().wm_create({
            let myself = self.clone();
            move |_msg| {
                info!("WM_CREATE");
                myself.wnd.hwnd().DragAcceptFiles(true);
                let _ = crate::utils::try_set_dark_mode(myself.wnd.hwnd());
                if let Ok(settings) = SETTINGS.read() {
                    let mut font = HFONT::CreateFont(
                        SIZE::new(0, settings.font.size),
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

                    myself.list_view.hwnd().SendMessage(
                        SetFont {
                            hfont: font.leak(),
                            redraw: true,
                        }
                        .as_generic_wm(),
                    );
                }

                unsafe {
                    match myself.list_view.hwnd().SetWindowSubclass(
                        Self::subclass_list_view,
                        0,
                        &myself as *const _ as _,
                    ) {
                        Ok(_) => {
                            info!("MainWindow: SetWindowSubclass: OK");
                        }
                        Err(e) => {
                            error!("MainWindow: SetWindowSubclass: {e}");
                        }
                    };
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
                                    myself.list_view.items().set_count(
                                        (myself.view.read().unwrap().as_ref().unwrap().line_count())
                                            as u32,
                                        None,
                                    );
                                    myself.wnd.set_text(format!("GORL - {f}").as_str());
                                    info!(
                                        "set {f}. lines = {}",
                                        myself.view.read().unwrap().as_ref().unwrap().line_count()
                                    );
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

        self.list_view.on().nm_custom_draw({
            let myself = self.clone();
            move |draw: &mut winsafe::NMLVCUSTOMDRAW| {
                match draw.mcd.dwDrawStage {
                    CDDS::PREPAINT => {
                        debug!("PREPAINT");
                        Ok(co::CDRF::NOTIFYITEMDRAW)
                    }
                    CDDS::ITEMPREPAINT => {
                        if let Ok(line) = myself.view.write().unwrap().as_mut().unwrap().get_line(draw.mcd.dwItemSpec as u64){
                            if let Some(highlight) = &myself.highlighter.matches(line.as_str()) {

                                let txt_clr = COLORREF::new(highlight.fg_color.0, highlight.fg_color.1, highlight.fg_color.2);
                                unsafe {
                                    *draw.clrText.as_mut() = txt_clr.raw();
                                }

                                let bg_clr = COLORREF::new(highlight.bg_color.0, highlight.bg_color.1, highlight.bg_color.2);
                                unsafe {
                                    *draw.clrTextBk.as_mut() = bg_clr.raw();
                                }

                                debug!("nm_custom_draw::ITEMPREPAINT::draw.mcd.dwItemSpec={} MATCHED;", draw.mcd.dwItemSpec);
                            }
                        }

                        Ok(co::CDRF::DODEFAULT)
                    }
                    _ => Ok(co::CDRF::DODEFAULT),
                }
            }
        });

        self.list_view.on().lvn_get_disp_info({
            let myself = self.clone();
            move |info| {
                if myself.view.read().is_ok_and(|o| o.is_none()) {
                    return Ok(());
                }

                //info.item.mask |= co::LVIF::PARAM;
                info.item.lParam = 1337;

                if info.item.mask.has(co::LVIF::TEXT) {
                    // is this a text request?
                    //println!("iItem={}; iSubItem={}; cColumns={};", info.item.iItem, info.item.iSubItem,info.item.cColumns);
                    let index = info.item.iItem as usize;
                    if info.item.iSubItem == 0 {
                        let (ptr, cch) = info.item.raw_pszText(); // retrieve raw pointer
                        let out_slice = unsafe { std::slice::from_raw_parts_mut(ptr, cch as _) };
                        out_slice
                            .iter_mut()
                            .zip(WString::from_str(format!("{}", index + 1)).as_slice())
                            .for_each(|(dest, src)| *dest = *src); // copy from our string to their buffer
                    } else {
                        let line_text = if let Ok(mut lock_res) = myself.view.write() {
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
                                let out_slice =
                                    unsafe { std::slice::from_raw_parts_mut(ptr, cch as _) };
                                out_slice
                                    .iter_mut()
                                    .zip(WString::from_str(text.as_str()).as_slice())
                                    .for_each(|(dest, src)| *dest = *src); // copy from our string to their buffer
                            }
                            r => error!("ERROR getting line: {:?}", r),
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
