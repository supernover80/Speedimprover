//! The eframe application: wires the state, the worker and the pages.

use std::path::{Path, PathBuf};

use egui::{CentralPanel, Frame, Margin, Panel};
use si_probe::{ProbeFactory, ScanResult};

use crate::settings::{AppPaths, Settings};
use crate::state::{AppState, UiEffect};
use crate::theme::{self, Palette};
use crate::widgets::toasts::Toasts;
use crate::worker::{WorkerCommand, WorkerConfig, WorkerEvent, WorkerHandle, spawn_worker};
use crate::{nav, pages};

pub struct SpeedImproverApp {
    pub state: AppState,
    worker: Option<WorkerHandle>,
    paths: Option<AppPaths>,
    styled_with: Option<Palette>,
    fonts_installed: bool,
}

/// `--fixture <path>` from the command line.
pub fn fixture_arg(args: impl IntoIterator<Item = String>) -> Option<PathBuf> {
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--fixture" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

fn load_last_scan(path: &Path) -> Option<ScanResult> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

impl SpeedImproverApp {
    pub fn new(cc: &eframe::CreationContext, paths: AppPaths, make_probe: ProbeFactory) -> Self {
        let mut state = AppState::new(Settings::load(&paths.config_file));
        let worker = spawn_worker(
            WorkerConfig {
                make_probe,
                wu_search: si_probe::search_windows_update_drivers,
                last_scan_path: paths.last_scan.clone(),
            },
            cc.egui_ctx.clone(),
        );
        // Show the previous scan at once (without a "Scan complete" toast),
        // then start a fresh one.
        if let Some(previous) = load_last_scan(&paths.last_scan) {
            state.apply_event(WorkerEvent::ScanDone(Box::new(previous)));
            state.toasts = Toasts::default();
        }
        state.request_scan();
        state.set_page(state.page);
        Self {
            state,
            worker: Some(worker),
            paths: Some(paths),
            styled_with: None,
            fonts_installed: false,
        }
    }

    /// No worker and no files: for tests.
    pub fn headless(state: AppState) -> Self {
        Self {
            state,
            worker: None,
            paths: None,
            styled_with: None,
            fonts_installed: false,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if !self.fonts_installed {
            theme::install_fonts(&ctx);
            self.fonts_installed = true;
        }
        if self.styled_with != Some(self.state.palette) {
            theme::apply(&ctx, &self.state.palette);
            self.styled_with = Some(self.state.palette);
        }
        self.state.now = ctx.input(|i| i.time);
        let palette = self.state.palette;

        Panel::left("sidebar")
            .exact_size(72.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(palette.card))
            .show(ui, |ui| nav::sidebar(ui, &mut self.state));
        CentralPanel::default()
            .frame(Frame::new().fill(palette.bg).inner_margin(Margin::same(20)))
            .show(ui, |ui| pages::show(ui, &mut self.state));
        pages::drivers::install_dialog(&ctx, &mut self.state);
        self.state.toasts.show(&ctx, &palette, self.state.now);
    }

    /// Applies worker events and carries out queued effects.
    fn pump(&mut self) {
        if let Some(worker) = &self.worker {
            while let Ok(event) = worker.rx.try_recv() {
                self.state.apply_event(event);
            }
        }
        for effect in std::mem::take(&mut self.state.effects) {
            match effect {
                UiEffect::Send(command) => {
                    if let Some(worker) = &self.worker {
                        let _ = worker.tx.send(command);
                    }
                }
                UiEffect::OpenUrl(url) => {
                    if let Err(e) = open::that_detached(&url) {
                        tracing::warn!("could not open {url}: {e}");
                    }
                }
                UiEffect::SaveSettings => {
                    if let Some(paths) = &self.paths {
                        if let Err(e) = self.state.settings.save(&paths.config_file) {
                            tracing::warn!("could not save settings: {e}");
                        }
                    }
                }
            }
        }
    }
}

impl eframe::App for SpeedImproverApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }

    fn on_exit(&mut self) {
        if let Some(worker) = &self.worker {
            let _ = worker.tx.send(WorkerCommand::Shutdown);
        }
    }
}
