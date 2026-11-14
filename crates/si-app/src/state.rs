//! Everything the UI shows, and how worker events change it. Pages mutate
//! this state and queue [`UiEffect`]s; the app carries the effects out, so
//! pages stay testable without a worker.

use si_core::drivers::DriverRow;
use si_core::findings::{Finding, health_score};
use si_core::frames::FrameStats;
use si_core::live::LiveHistory;
use si_core::rules::analyze;
use si_core::tier::{HardwareProfile, hardware_profile};
use si_probe::catalog::CatalogSource;
use si_probe::{ScanResult, ScanStep};

use crate::nav::Page;
use crate::settings::Settings;
use crate::theme::{Palette, ThemeChoice};
use crate::widgets::toasts::{ToastKind, Toasts};
use crate::worker::{WorkerCommand, WorkerEvent};

/// Live samples kept for the charts (two minutes).
pub const LIVE_HISTORY: usize = 120;

#[derive(Debug, Clone, PartialEq)]
pub enum UiEffect {
    Send(WorkerCommand),
    OpenUrl(String),
    SaveSettings,
}

pub struct AppState {
    pub page: Page,
    pub settings: Settings,
    pub palette: Palette,
    pub scan: Option<ScanResult>,
    /// The step currently running, while a scan is in progress.
    pub scan_step: Option<ScanStep>,
    pub findings: Vec<Finding>,
    pub health: u8,
    pub hardware: Option<HardwareProfile>,
    pub live: LiveHistory,
    pub live_running: bool,
    pub fps: Option<FrameStats>,
    pub frame_times: Vec<f64>,
    pub fps_target: Option<String>,
    pub drivers: Vec<DriverRow>,
    pub drivers_checking: bool,
    pub drivers_checked_at: Option<i64>,
    pub catalog_source: Option<CatalogSource>,
    pub expanded_finding: Option<&'static str>,
    pub selected_game: Option<String>,
    pub driver_class_filter: Option<String>,
    pub toasts: Toasts,
    pub effects: Vec<UiEffect>,
    /// egui time in seconds, updated every frame.
    pub now: f64,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        Self {
            page: Page::default(),
            palette: Palette::for_choice(settings.theme),
            settings,
            scan: None,
            scan_step: None,
            findings: Vec::new(),
            health: 100,
            hardware: None,
            live: LiveHistory::new(LIVE_HISTORY),
            live_running: false,
            fps: None,
            frame_times: Vec::new(),
            fps_target: None,
            drivers: Vec::new(),
            drivers_checking: false,
            drivers_checked_at: None,
            catalog_source: None,
            expanded_finding: None,
            selected_game: None,
            driver_class_filter: None,
            toasts: Toasts::default(),
            effects: Vec::new(),
            now: 0.0,
        }
    }

    pub fn apply_event(&mut self, e: WorkerEvent) {
        match e {
            WorkerEvent::ScanProgress(step) => self.scan_step = Some(step),
            WorkerEvent::ScanDone(result) => {
                self.scan_step = None;
                self.findings = analyze(&result.snapshot, &result.games);
                self.health = health_score(&self.findings);
                self.hardware = Some(hardware_profile(&result.snapshot));
                self.toasts
                    .push("Scan complete", ToastKind::Success, self.now);
                if !result.warnings.is_empty() {
                    self.toasts.push(
                        format!("Some data was unavailable ({})", result.warnings.len()),
                        ToastKind::Info,
                        self.now,
                    );
                }
                if self.drivers_checked_at.is_none() && !self.drivers_checking {
                    self.request_driver_check();
                }
                self.scan = Some(*result);
            }
            WorkerEvent::Sample(sample) => self.live.push(sample),
            WorkerEvent::Frames { stats, frame_times } => {
                self.fps = Some(stats);
                self.frame_times = frame_times;
            }
            WorkerEvent::Drivers {
                rows,
                catalog,
                checked_at_unix,
            } => {
                self.drivers = rows;
                self.catalog_source = Some(catalog);
                self.drivers_checked_at = Some(checked_at_unix);
                self.drivers_checking = false;
            }
            WorkerEvent::Error(message) => self.toasts.push(message, ToastKind::Error, self.now),
        }
    }

    pub fn send(&mut self, command: WorkerCommand) {
        self.effects.push(UiEffect::Send(command));
    }

    pub fn open_url(&mut self, url: impl Into<String>) {
        self.effects.push(UiEffect::OpenUrl(url.into()));
    }

    pub fn request_scan(&mut self) {
        self.scan_step = Some(ScanStep::Hardware);
        self.send(WorkerCommand::Scan);
    }

    pub fn request_driver_check(&mut self) {
        self.drivers_checking = true;
        let catalog_url = self.settings.catalog_url.clone();
        self.send(WorkerCommand::CheckDrivers { catalog_url });
    }

    /// Navigates; live sampling runs only while the Dashboard or Live page is shown.
    pub fn set_page(&mut self, p: Page) {
        self.page = p;
        let wants_live = matches!(p, Page::Dashboard | Page::Live);
        if wants_live && !self.live_running {
            self.live_running = true;
            self.send(WorkerCommand::StartLive);
        } else if !wants_live && self.live_running {
            self.live_running = false;
            self.send(WorkerCommand::StopLive);
        }
    }

    pub fn set_theme(&mut self, t: ThemeChoice) {
        self.settings.theme = t;
        self.palette = Palette::for_choice(t);
        self.effects.push(UiEffect::SaveSettings);
    }
}
