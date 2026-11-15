//! The background worker. It owns the probe (built on its own thread, since
//! WMI/COM objects are not `Send`), so the UI thread never blocks.

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use si_core::drivers::{AvailableDriverUpdate, DriverRow, evaluate_drivers};
use si_core::frames::{FrameSample, FrameStats, FrameWindow, frame_stats};
use si_core::live::LiveSample;
use si_probe::catalog::{CatalogSource, load_catalog};
use si_probe::presentmon::PresentMonSession;
use si_probe::{ProbeError, ProbeFactory, ScanResult, ScanStep, SystemProbe};

/// Live samples and FPS updates are sent this often.
const TICK: Duration = Duration::from_millis(1000);
/// FPS statistics cover the last five seconds of frames.
const FRAME_WINDOW_MS: f64 = 5000.0;

#[derive(Debug, Clone, PartialEq)]
pub enum WorkerCommand {
    Scan,
    StartLive,
    StopLive,
    CheckDrivers {
        catalog_url: String,
    },
    StartFps {
        presentmon: PathBuf,
        process_name: String,
    },
    StopFps,
    Shutdown,
}

#[derive(Debug)]
pub enum WorkerEvent {
    ScanProgress(ScanStep),
    ScanDone(Box<ScanResult>),
    Sample(LiveSample),
    Frames {
        stats: FrameStats,
        frame_times: Vec<f64>,
        frames: Vec<FrameSample>,
    },
    Drivers {
        rows: Vec<DriverRow>,
        catalog: CatalogSource,
        checked_at_unix: i64,
    },
    Error(String),
}

pub type WuSearchFn = fn() -> Result<Vec<AvailableDriverUpdate>, ProbeError>;

pub struct WorkerConfig {
    pub make_probe: ProbeFactory,
    pub wu_search: WuSearchFn,
    /// Where each completed scan is saved, so the next launch shows it instantly.
    pub last_scan_path: PathBuf,
}

pub struct WorkerHandle {
    pub tx: Sender<WorkerCommand>,
    pub rx: Receiver<WorkerEvent>,
}

pub fn spawn_worker(cfg: WorkerConfig, repaint: egui::Context) -> WorkerHandle {
    let (tx, commands) = unbounded();
    let (events, rx) = unbounded();
    let spawned = thread::Builder::new()
        .name("speedimprover-worker".into())
        .spawn(move || {
            let probe = (cfg.make_probe)();
            Worker {
                probe,
                events: Emitter { events, repaint },
                wu_search: cfg.wu_search,
                last_scan_path: cfg.last_scan_path,
                last_scan: None,
                pending_driver_check: None,
                live: false,
                fps: None,
            }
            .run(commands);
        });
    if let Err(e) = spawned {
        tracing::error!("could not start the worker thread: {e}");
    }
    WorkerHandle { tx, rx }
}

/// Sends events and wakes the UI.
#[derive(Clone)]
struct Emitter {
    events: Sender<WorkerEvent>,
    repaint: egui::Context,
}

impl Emitter {
    fn send(&self, event: WorkerEvent) {
        let _ = self.events.send(event);
        self.repaint.request_repaint();
    }
}

struct Worker {
    probe: Box<dyn SystemProbe>,
    events: Emitter,
    wu_search: WuSearchFn,
    last_scan_path: PathBuf,
    last_scan: Option<ScanResult>,
    /// A driver check asked for before the first scan finished.
    pending_driver_check: Option<String>,
    live: bool,
    fps: Option<(PresentMonSession, FrameWindow)>,
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

impl Worker {
    fn run(mut self, commands: Receiver<WorkerCommand>) {
        let mut next_tick = Instant::now() + TICK;
        loop {
            let active = self.live || self.fps.is_some();
            let command = if active {
                let wait = next_tick.saturating_duration_since(Instant::now());
                match commands.recv_timeout(wait) {
                    Ok(c) => Some(c),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            } else {
                match commands.recv() {
                    Ok(c) => Some(c),
                    Err(_) => return,
                }
            };
            match command {
                Some(WorkerCommand::Shutdown) => return,
                Some(c) => self.handle(c),
                None => {}
            }
            if (self.live || self.fps.is_some()) && Instant::now() >= next_tick {
                self.tick();
                next_tick = Instant::now() + TICK;
            }
        }
    }

    fn handle(&mut self, command: WorkerCommand) {
        match command {
            WorkerCommand::Scan => self.scan(),
            WorkerCommand::StartLive => self.live = true,
            WorkerCommand::StopLive => self.live = false,
            WorkerCommand::CheckDrivers { catalog_url } => {
                if self.last_scan.is_some() {
                    self.check_drivers(catalog_url);
                } else {
                    self.pending_driver_check = Some(catalog_url);
                }
            }
            WorkerCommand::StartFps {
                presentmon,
                process_name,
            } => match PresentMonSession::start(&presentmon, &process_name) {
                Ok(session) => self.fps = Some((session, FrameWindow::new(FRAME_WINDOW_MS))),
                Err(e) => self.events.send(WorkerEvent::Error(format!(
                    "Could not start PresentMon: {e}"
                ))),
            },
            WorkerCommand::StopFps => self.fps = None,
            WorkerCommand::Shutdown => {}
        }
    }

    fn scan(&mut self) {
        let events = self.events.clone();
        let result = self
            .probe
            .scan(&mut |step| events.send(WorkerEvent::ScanProgress(step)));
        self.save_last_scan(&result);
        self.last_scan = Some(result.clone());
        self.events.send(WorkerEvent::ScanDone(Box::new(result)));
        if let Some(url) = self.pending_driver_check.take() {
            self.check_drivers(url);
        }
    }

    fn save_last_scan(&self, result: &ScanResult) {
        let saved = self
            .last_scan_path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .map_err(anyhow::Error::from)
            .and_then(|()| Ok(serde_json::to_string(result)?))
            .and_then(|json| Ok(std::fs::write(&self.last_scan_path, json)?));
        if let Err(e) = saved {
            tracing::warn!("could not save the last scan: {e}");
        }
    }

    /// Catalog + Windows Update on their own thread (they can take a minute).
    fn check_drivers(&self, catalog_url: String) {
        let snapshot = self
            .last_scan
            .as_ref()
            .map(|r| r.snapshot.clone())
            .unwrap_or_default();
        let events = self.events.clone();
        let wu_search = self.wu_search;
        let spawned = thread::Builder::new()
            .name("speedimprover-drivers".into())
            .spawn(move || {
                let (catalog, source) = load_catalog(&catalog_url);
                let offered = match wu_search() {
                    Ok(list) => list,
                    Err(e) => {
                        events.send(WorkerEvent::Error(format!(
                            "Windows Update check failed: {e}"
                        )));
                        Vec::new()
                    }
                };
                events.send(WorkerEvent::Drivers {
                    rows: evaluate_drivers(&snapshot, &catalog, &offered),
                    catalog: source,
                    checked_at_unix: now_unix(),
                });
            });
        if let Err(e) = spawned {
            self.events.send(WorkerEvent::Error(format!(
                "Could not start the driver check: {e}"
            )));
        }
    }

    fn tick(&mut self) {
        if self.live {
            self.events.send(WorkerEvent::Sample(self.probe.sample()));
        }
        let mut stopped = false;
        if let Some((session, window)) = &mut self.fps {
            session.drain_into(window);
            let frame_times = window.as_slice();
            if let Some(stats) = frame_stats(&frame_times) {
                self.events.send(WorkerEvent::Frames {
                    stats,
                    frame_times,
                    frames: window.samples(),
                });
            }
            stopped = !session.is_running();
        }
        if stopped {
            self.fps = None;
            self.events.send(WorkerEvent::Error(
                "FPS capture stopped (the game closed, or PresentMon needs administrator rights)"
                    .into(),
            ));
        }
    }
}
