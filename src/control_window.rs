use log::{error, info};
use std::sync::Arc;

use winsafe::co::{BS, CHARSET, CLIP, FW, OUT_PRECIS, PITCH, QUALITY, SS, WS, WS_EX};
use winsafe::gui::{Horz, LabelOpts, Vert};
use winsafe::{gui, HFONT, SIZE};

use crate::main_window::GorlMainWindow;
use crate::SETTINGS;
use winsafe::msg::wm::SetFont;
use winsafe::prelude::{
    gdi_Hfont, shell_Hwnd, user_Hwnd, GuiEvents, GuiNativeControlEvents, GuiParent, GuiThread,
    GuiWindow, GuiWindowText, MsgSend,
};

#[derive(Clone)]
pub(crate) struct ControlPanel {
    pub(crate) wnd: gui::WindowMain,
    new_log_wnd_btn: gui::Button,
    rt_handle: Arc<tokio::runtime::Runtime>,
    mem_label: gui::Label,
}

impl ControlPanel {
    pub fn new(rt_handle: Arc<tokio::runtime::Runtime>) -> Self {
        info!("Creating Main Window. Settings = {:?}", SETTINGS.read());

        let win_width = 400;

        let wnd = gui::WindowMain::new(
            // instantiate the window manager
            gui::WindowMainOpts {
                title: "GORL - Control Panel".to_owned(),
                size: (win_width, 56),
                class_name: "GorlMainWindow_ControlPanel".to_owned(),
                style: gui::WindowMainOpts::default().style,
                ex_style: WS_EX::TOPMOST,
                ..Default::default() // leave all other options as default
            },
        );

        let btn_width = 150;

        let new_log_wnd_btn = gui::Button::new(
            &wnd,
            gui::ButtonOpts {
                height: 36,
                width: btn_width,
                text: "New 🪵🪟".to_owned(),
                position: (10, 10),
                button_style: BS::DEFPUSHBUTTON | BS::PUSHBUTTON | BS::FLAT,
                resize_behavior: (Horz::None, Vert::None),
                ..Default::default()
            },
        );

        let lbl_with = win_width - btn_width - 40;

        let mem_label = gui::Label::new(
            &wnd,
            LabelOpts {
                text: "🐏 5 MB".to_string(),
                position: ((win_width - lbl_with - 10) as i32, 10),
                size: (lbl_with, 36),
                label_style: SS::RIGHT,
                window_style: WS::BORDER | WS::CHILD | WS::VISIBLE,
                resize_behavior: (Horz::None, Vert::None),
                ..Default::default()
            },
        );

        let mut new_self = Self {
            wnd,
            new_log_wnd_btn,
            rt_handle,
            mem_label,
        };
        new_self.events();
        new_self
    }

    fn events(&mut self) {
        self.wnd.on().wm_create({
            let myself = self.clone();
            move |_msg| {
                info!("CONTROL_PANEL: WM_CREATE");
                myself.wnd.hwnd().DragAcceptFiles(true);

                let _ = crate::utils::try_set_dark_mode(myself.wnd.hwnd());

                let mut font = HFONT::CreateFont(
                    SIZE::new(0, 30),
                    0,
                    0,
                    FW::MEDIUM,
                    false,
                    false,
                    false,
                    CHARSET::DEFAULT,
                    OUT_PRECIS::DEFAULT,
                    CLIP::DEFAULT_PRECIS,
                    QUALITY::CLEARTYPE,
                    PITCH::FIXED,
                    "Verdana",
                )?;

                myself.new_log_wnd_btn.hwnd().SendMessage(
                    SetFont {
                        hfont: font.leak(),
                        redraw: true,
                    }
                    .as_generic_wm(),
                );

                let mut font = HFONT::CreateFont(
                    SIZE::new(0, 30),
                    0,
                    0,
                    FW::MEDIUM,
                    false,
                    false,
                    false,
                    CHARSET::DEFAULT,
                    OUT_PRECIS::DEFAULT,
                    CLIP::DEFAULT_PRECIS,
                    QUALITY::CLEARTYPE,
                    PITCH::DEFAULT,
                    "Verdana",
                )?;

                myself.mem_label.hwnd().SendMessage(
                    SetFont {
                        hfont: font.leak(),
                        redraw: true,
                    }
                    .as_generic_wm(),
                );

                Ok(0)
            }
        });

        self.new_log_wnd_btn.on().bn_clicked({
            let myself = self.clone();
            move || {
                info!("CONTROL_PANEL: NEW LOG CLICKED");
                myself.rt_handle.spawn_blocking(|| {
                    let my = GorlMainWindow::new(); // instantiate our main window
                    if let Err(e) = my.wnd.run_main(None) {
                        // ... and run it
                        error!("{}", e);
                    }
                });
                Ok(())
            }
        });

        self.rt_handle.spawn({
            let myself = self.clone();
            async move {
                loop {
                    let text = if let Some(mem_info) = Self::get_mem_info() {
                        format!("🐏 {} ", mem_info)
                    } else {
                        "🐏 😭 ".to_string()
                    };

                    myself.wnd.run_ui_thread({
                        let cloned_self = myself.clone();
                        move || {
                            cloned_self.mem_label.set_text(text.as_str());
                            Ok(())
                        }
                    });

                    tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.3)).await;
                }
            }
        });
    }

    fn get_mem_info() -> Option<String> {
        memory_stats::memory_stats()
            .map(|stats| humansize::format_size(stats.virtual_mem, humansize::WINDOWS))
    }
}
