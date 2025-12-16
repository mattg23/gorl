use skia_safe::{Canvas, Contains, Point, Rect};
use winit::event::{ElementState, MouseButton, WindowEvent};

use crate::theme::{self, GorlColor, solid};

pub trait Component {
    fn draw(&mut self, canvas: &Canvas, rect: &Rect);
    fn handle_event(&mut self, event: &WindowEvent) -> bool;
    fn get_rect(&self) -> Option<Rect>;
    fn contains(&self, point: &Point) -> Option<bool> {
        Some(self.get_rect()?.contains(point))
    }
}

pub struct Button {
    rect: Option<Rect>,
    label: String,
    on_click: Box<dyn Fn(&Self) -> ()>,
}

impl Button {
    pub fn new(label: String, on_click: Box<dyn Fn(&Self) -> ()>) -> Self {
        Self {
            rect: None,
            label,
            on_click,
        }
    }

    pub fn set_label(&mut self, s: String) {
        self.label = s;
    }
}

impl Component for Button {
    fn draw(&mut self, canvas: &Canvas, rect: &Rect) {
        canvas.draw_rect(&rect, &solid(GorlColor::CtrlBg));
        let txt = self.label.as_str();
        let txt_paint = solid(GorlColor::CtrlFg);

        let font = theme::ui_font();
        let (_, txt_r) = font.measure_str(txt, Some(&txt_paint));
        canvas.draw_str(
            txt,
            (
                (rect.center_x() - (txt_r.width() / 2.0)),
                (rect.center_y() + (txt_r.height() / 2.0)),
            ),
            &theme::ui_font(),
            &txt_paint,
        );

        self.rect = Some(*rect);
    }

    fn handle_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } if *button == MouseButton::Left && *state == ElementState::Released => {
                (self.on_click)(self);
                true
            }
            _ => false,
        }
    }

    fn get_rect(&self) -> Option<Rect> {
        self.rect
    }
}
