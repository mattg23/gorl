mod control_window;
mod highlighter;
mod lineview;
mod main_window;
mod search;
mod settings;
mod utils;

use egui::{DroppedFile, FontId, Id, RichText};
use egui_extras::{Column, TableBuilder};
use lazy_static::lazy_static;
use lineview::LineBasedFileView;
use log::error;
use rand::Rng;
use tracing::info;

use std::{fs::File, sync::RwLock};
lazy_static! {
    static ref SETTINGS: RwLock<settings::Settings> = RwLock::new(settings::Settings::new());
}

struct GorlApp {
    logs: Vec<GorlLogWindow>,
    dropped_files: Vec<DroppedFile>,
}

impl GorlApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

        cc.egui_ctx.set_fonts(fonts);

        Self {
            dropped_files: vec![],
            logs: vec![],
        }
    }

    fn open_file(&self, path: &str) -> anyhow::Result<LineBasedFileView<File>> {
        let bf = std::time::SystemTime::now();
        let view = LineBasedFileView::new(File::open(path)?, path)?;
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

    fn get_mem_info() -> Option<String> {
        memory_stats::memory_stats()
            .map(|stats| humansize::format_size(stats.virtual_mem, humansize::WINDOWS))
    }
}

impl eframe::App for GorlApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            if (!i.raw.dropped_files.is_empty()) {
                for f in &i.raw.dropped_files {
                    if let Some(f) = &f.path {
                        if let Some(f) = f.to_str() {
                            match GorlLogWindow::new(f) {
                                Ok(window) => self.logs.push(window),
                                Err(e) => error!("{}", anyhow::anyhow!(e)),
                            }
                        }
                    }
                }
            }
        });

        egui::TopBottomPanel::top("my_panel")
            .frame(egui::Frame {
                inner_margin: egui::Margin::same(8f32),
                ..Default::default()
            })
            .show(ctx, |ui| {
                let new_window_btn = ui.button(
                    RichText::new(format!("New {} Window", egui_phosphor::regular::LOG))
                        .font(FontId::proportional(24.)),
                );

                if let Some(stats) = Self::get_mem_info() {
                    ui.label(RichText::new(format!("🐏 {} ", stats)));
                }
            });

        if !self.logs.is_empty() {
            for window in self.logs.iter_mut() {
                egui::Window::new(window.title())
                    .id(window.id())
                    .movable(true)
                    .scroll(true)
                    .resizable(true)
                    .collapsible(true)
                    .default_open(true)                    
                    .show(ctx, |ui| {
                        window.draw(ctx, ui);
                    });
            }
        }
    }
}

struct GorlLogWindow {
    view: LineBasedFileView<File>,
    id: Id,
}

impl GorlLogWindow {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let mut rng = rand::thread_rng();
        let id = Id::new(rng.gen::<i64>());

        let view = GorlLogWindow::open_file(path)?;

        Ok(Self { view, id })
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn open_file(path: &str) -> anyhow::Result<LineBasedFileView<File>> {
        let bf = std::time::SystemTime::now();
        let view = LineBasedFileView::new(File::open(path)?, path)?;
        let now = std::time::SystemTime::now();

        match now.duration_since(bf) {
            Ok(elapsed) => {
                info!(
                    "Indexed {} chunks from {path} in {}s",
                    view.page_count(),
                    elapsed.as_secs_f64()
                );

                Ok(view)
            }
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }

    pub fn title(&self) -> String {
        self.view.formatted_info()
    }

    fn draw(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let available_height = ui.available_height();

        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        let mut table = TableBuilder::new(ui)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .striped(true)
            .column(Column::auto())
            .column(Column::remainder().at_least(40.0).clip(false))
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height);

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    egui::Sides::new().show(
                        ui,
                        |ui| {
                            ui.strong("Line");
                        },
                        |ui| {},
                    );
                });
                header.col(|ui| {
                    ui.strong("Text");
                });
            })
            .body(|mut body| {
                body.rows(text_height, self.view.line_count() as usize, |mut row| {
                    let row_index = row.index();

                    row.col(|ui| {
                        ui.label((row_index + 1).to_string());
                    });

                    row.col(|ui| {
                        let text = match self.view.get_line(row_index as u64) {
                            Ok(s) => s,
                            Err(s) => s,
                        };

                        ui.label(text);
                    });
                });
            });
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 480.0]),
        ..Default::default() //.with_drag_and_drop(true),
    };

    // let rt = tokio::runtime::Builder::new_multi_thread()
    //     .enable_all()
    //     .max_blocking_threads(SETTINGS.read().unwrap().max_nb_of_ui_threads) // basically the limit of log file one can open
    //     .build()
    //     .unwrap();

    // let rt_handle = Arc::new(rt);

    // let outer_handle = rt_handle.clone();

    // let fst = outer_handle.spawn_blocking(move || {
    //     let my = ControlPanel::new(rt_handle); // instantiate our main window
    //     if let Err(e) = my.wnd.run_main(None) {
    //         // ... and run it
    //         error!("{}", e);
    //     }
    // });

    // outer_handle.block_on(async move {
    //     let _ = fst.await;
    // });

    _ = eframe::run_native(
        "GORL",
        options,
        Box::new(|cc| Ok(Box::<GorlApp>::new(GorlApp::new(cc)))),
    );

    Ok(())
}
