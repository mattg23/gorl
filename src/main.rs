mod common;
mod highlighter;
mod lineview;
mod search;
mod settings;
mod utils;

use anyhow::anyhow;
use lazy_static::lazy_static;
use rand::Rng;
use skia_safe::{AlphaType, ColorType, ImageInfo, Rect};
use std::num::NonZeroU32;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::HashMap, rc::Rc};
use tracing::{Instrument, Span, error};
use winit::{
    application::ApplicationHandler,
    dpi::Size,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};
mod components;
mod theme;
use crate::common::{GorlMsg, WindowId};
use crate::components::{Button, Component};
use crate::theme::{CTRL_FT_SIZE, GorlColor, solid};

lazy_static! {
    static ref SETTINGS: RwLock<settings::Settings> = RwLock::new(settings::Settings::new());
}

struct Gorl {
    main_window: Option<Rc<Window>>,
    context: Option<softbuffer::Context<Rc<Window>>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    button_new: Button,
}

impl Gorl {
    pub fn new() -> Self {
        Self {
            main_window: None,
            context: None,
            surface: None,
            button_new: Button::new(
                "Button Component".to_string(),
                Box::new(|s| {
                    dbg!(s.get_rect());
                }),
            ),
        }
    }

    fn draw(&mut self) -> anyhow::Result<()> {
        if self.main_window.is_none() {
            return Err(anyhow::anyhow!("no window to draw into"));
        }

        let p_size = self.main_window.as_ref().unwrap().inner_size();
        let (win_w, win_h) = (
            NonZeroU32::new(p_size.width).ok_or(anyhow!("window width is <= 0"))?,
            NonZeroU32::new(p_size.height).ok_or(anyhow!("window heigth is <= 0"))?,
        );
        let mut surface = self
            .surface
            .as_mut()
            .ok_or(anyhow!("could not get surface"))?;

        surface.resize(win_w, win_h);

        let (w, h) = (win_w.get(), win_h.get());
        let mut buffer = surface
            .buffer_mut()
            .map_err(|e| anyhow!("could not get mut buffer. reason: {e:?}"))?;

        let row_bytes = w * 4;

        let len = buffer.len() * std::mem::size_of::<u32>();

        let skia_buf: &mut [u8] =
            unsafe { std::slice::from_raw_parts_mut(buffer.as_mut_ptr() as *mut u8, len) };

        let info = ImageInfo::new(
            (w as i32, h as i32),
            ColorType::BGRA8888,
            AlphaType::Unpremul,
            None,
        );

        let mut surface =
            skia_safe::surfaces::wrap_pixels(&info, skia_buf, row_bytes as usize, None).unwrap();

        let canvas = surface.canvas();
        canvas.clear(theme::get_color(GorlColor::WndBg));

        let btn_rect = Rect::from_xywh(10.0, 10.0, (w as f32 / 2.0) - 10.0, (h as f32) - 20.0);

        self.button_new.draw(canvas, &btn_rect);
        buffer
            .present()
            .map_err(|_| anyhow!("could not present buffer"))?;
        Ok(())
    }
}
impl ApplicationHandler for Gorl {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.main_window.is_some() {
            return;
        }
        let w_size = winit::dpi::PhysicalSize::new(420, 60);
        let w = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title("GORL 🪵🪟 - Control Window")
                    .with_resizable(true)
                    .with_inner_size(w_size)
                    .with_min_inner_size(w_size), // .with_max_inner_size(w_size),
            )
            .expect("could not create window");
        let w = Rc::new(w);
        self.context =
            Some(softbuffer::Context::new(w.clone()).expect("Cannot create drawing context"));
        self.surface = Some(
            softbuffer::Surface::new(self.context.as_ref().unwrap(), w.clone())
                .expect("cannot create drawing surface"),
        );
        self.main_window = Some(w);
    }
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        //dbg!(&event);
        match event {
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                dbg!(position);
                dbg!(
                    self.button_new
                        .contains(&skia_safe::Point::new(position.x as f32, position.y as f32))
                );
            }
            WindowEvent::MouseInput { .. } => {
                self.button_new.handle_event(&event);
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.draw() {
                    error!("{}", anyhow::format_err!("{}", e));
                }
            }
            _ => (),
        }
    }
}
fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    theme::initialize_theme();
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut gorl = Gorl::new();
    event_loop.run_app(&mut gorl);

    Ok(())
}
