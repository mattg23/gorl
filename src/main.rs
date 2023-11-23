mod control_window;
mod highlighter;
mod lineview;
mod main_window;
mod search;
mod settings;
mod utils;

use crate::control_window::ControlPanel;
use lazy_static::lazy_static;
use log::error;
use std::sync::{Arc, RwLock};

lazy_static! {
    static ref SETTINGS: RwLock<settings::Settings> = RwLock::new(settings::Settings::new());
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(SETTINGS.read().unwrap().max_nb_of_ui_threads) // basically the limit of log file one can open
        .build()
        .unwrap();

    let rt_handle = Arc::new(rt);

    let outer_handle = rt_handle.clone();

    let fst = outer_handle.spawn_blocking(move || {
        let my = ControlPanel::new(rt_handle); // instantiate our main window
        if let Err(e) = my.wnd.run_main(None) {
            // ... and run it
            error!("{}", e);
        }
    });

    outer_handle.block_on(async move {
        let _ = fst.await;
    });

    Ok(())
}
