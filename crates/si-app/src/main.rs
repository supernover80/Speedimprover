#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! SpeedImprover: finds what is costing FPS on this PC.

use std::path::Path;
use std::sync::Mutex;

use si_app::app::{SpeedImproverApp, fixture_arg};
use si_app::settings::AppPaths;
use si_probe::ProbeFactory;
use si_probe::fixture::FixtureProbe;

fn init_logging(log_file: &Path) {
    let file = log_file
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::File::create(log_file));
    match file {
        Ok(file) => tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(Mutex::new(file))
            .init(),
        Err(_) => tracing_subscriber::fmt().init(),
    }
}

fn main() -> eframe::Result {
    let paths = AppPaths::from_project_dirs()
        .unwrap_or_else(|| AppPaths::in_dir(&std::env::temp_dir().join("SpeedImprover")));
    init_logging(&paths.log_file);

    let make_probe: ProbeFactory = match fixture_arg(std::env::args()) {
        Some(path) => Box::new(move || match FixtureProbe::load(&path) {
            Ok(probe) => Box::new(probe),
            Err(e) => {
                tracing::error!("could not load fixture {}: {e}", path.display());
                si_probe::default_probe()
            }
        }),
        None => Box::new(si_probe::default_probe),
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SpeedImprover")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1100.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "SpeedImprover",
        options,
        Box::new(|cc| Ok(Box::new(SpeedImproverApp::new(cc, paths, make_probe)))),
    )
}
