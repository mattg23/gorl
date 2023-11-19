use std::rc::Rc;
use std::sync::Arc;
use log::{error, info};
use winsafe::{co, gui, HFONT, SIZE};
use winsafe::co::{BS, CHARSET, CLIP, FW, OUT_PRECIS, PITCH, QUALITY};
use winsafe::gui::{Horz, Vert};
use winsafe::msg::wm::SetFont;
use winsafe::prelude::{gdi_Hfont, GuiEvents, GuiNativeControlEvents, GuiParent, GuiWindow, MsgSend, shell_Hwnd, user_Hwnd};
use crate::main_window::GorlMainWindow;
use crate::SETTINGS;

#[derive(Clone)]
pub(crate) struct ControlPanel {
    pub(crate) wnd: gui::WindowMain,
    new_log_wnd_btn: gui::Button,
    rt_handle: Arc<tokio::runtime::Runtime>
}


impl ControlPanel {
    pub fn new(rt_handle:Arc<tokio::runtime::Runtime>) -> Self {
        info!("Creating Main Window. Settings = {:?}", SETTINGS.read());

        let wnd = gui::WindowMain::new(
            // instantiate the window manager
            gui::WindowMainOpts {
                title: "GORL - Control Panel".to_owned(),
                size: (400, 56),
                class_name: "GorlMainWindow_ControlPanel".to_owned(),
                style: gui::WindowMainOpts::default().style,
                ..Default::default() // leave all other options as default
            },
        );

        let new_log_wnd_btn = gui::Button::new(
            &wnd,
            gui::ButtonOpts {
                height: 36,
                width: 150,
                text: "New 🪵🪟".to_owned(),
                position: (10, 10),
                button_style: BS::DEFPUSHBUTTON | BS::PUSHBUTTON | BS::FLAT,
                resize_behavior: (Horz::None, Vert::None),
                ..Default::default()
            },
        );

        let mut new_self = Self {
            wnd,
            new_log_wnd_btn,
            rt_handle
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

                let mut font = HFONT::CreateFont(
                    SIZE::new(16, 24),
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
                    }.as_generic_wm(),
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
        })
    }
}