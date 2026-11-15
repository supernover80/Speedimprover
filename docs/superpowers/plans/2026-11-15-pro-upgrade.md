# SpeedImprover Pro Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** CPU/GPU temperatures, an exact per-frame bottleneck report, one-click Windows Update driver installs, and a professional look.

**Architecture:** Pure logic in `si-core` (temperature selection lives in `si-probe::temps` because it parses WMI rows; bottleneck report and install-report types in `si-core`), Windows readers in `si-probe::win`, UI in `si-app`. The elevated installer is the same executable started with `--install-drivers`.

**Tech Stack:** as before, plus `windows` features `Wdk_Graphics_Direct3D`, `Win32_System_Restore`, `Win32_UI_Shell`, `Win32_System_Threading`.

**Spec:** `docs/superpowers/specs/2026-11-15-pro-upgrade-design.md` (extends `2026-11-14-speedimprover-design.md`)

## Global Constraints

- All constraints of `2026-11-14-speedimprover.md` still hold, except "read-only": installing Windows Update drivers after explicit confirmation is now allowed (spec §2).
- The elevated helper accepts only GUID update IDs and a result path; it never runs anything else.
- No new crates; only new `windows` features.
- `cargo fmt`, Linux clippy, Windows-target clippy (local cross-check) and `cargo test --workspace` pass at every commit.

## Review Focus

1. **No sensors** (VMs, desktops without LibreHardwareMonitor, AMD/Intel without D3DKMT perf data): temperatures show `—`, nothing panics — `temps_absent_are_none` (Task 1).
2. **PresentMon without busy columns** (older versions): frame mode is skipped, load mode is used — `frames_without_busy_use_load_mode` (Task 2).
3. **Hostile helper arguments** (`--install-drivers x.json "; calc"`): rejected — `install_args_reject_non_guid` (Task 3).
4. **UAC declined / not Windows**: a clear "Installation cancelled" error, `drivers_installing` cleared — `install_failure_clears_state` (Task 3).
5. **Light/dark/system theme** at 1100×700: no overflow — visual check (Task 4).

---

### Task 1: Temperatures

**Files:** modify `crates/si-core/src/live.rs`, `crates/si-core/src/model.rs` (`GpuInfo.luid: Option<u64>`), `crates/si-probe/src/{lib,mapping}.rs`, `crates/si-probe/src/win/{mod,dxgi,pdh}.rs`, `crates/si-probe/Cargo.toml`, `crates/si-probe/src/fixture.rs`; create `crates/si-probe/src/temps.rs`, `crates/si-probe/src/win/{gpu_perf,lhm}.rs`; test `crates/si-probe/tests/temps.rs`.

**Interfaces (produces):**
- `LiveSample.cpu_temp_c: Option<f32>`, `LiveSample.cpu_temp_exact: bool`
- `temps::LhmSensor { identifier: Option<String>, name: Option<String>, sensor_type: Option<String>, value: Option<f32> }` (serde `rename = "Sensor"`, PascalCase)
- `temps::cpu_temp_from_sensors(&[LhmSensor]) -> Option<f32>`, `temps::gpu_temp_from_sensors(&[LhmSensor]) -> Option<f32>`, `temps::thermal_zone_celsius(&[(String, f64)]) -> Option<f32>`, `temps::deci_celsius(raw: u32) -> Option<f32>` (0 ⇒ None)

- [ ] **Step 1: failing tests** (`tests/temps.rs`)

```rust
use si_probe::temps::*;
fn s(id: &str, name: &str, t: &str, v: f32) -> LhmSensor {
    LhmSensor { identifier: Some(id.into()), name: Some(name.into()), sensor_type: Some(t.into()), value: Some(v) }
}
#[test] fn picks_cpu_package() {
    let rows = [s("/intelcpu/0/temperature/0", "CPU Core #1", "Temperature", 70.0),
                s("/intelcpu/0/temperature/8", "CPU Package", "Temperature", 74.0),
                s("/intelcpu/0/load/0", "CPU Total", "Load", 99.0)];
    assert_eq!(cpu_temp_from_sensors(&rows), Some(74.0));
}
#[test] fn amd_tctl_and_fallback_max() {
    assert_eq!(cpu_temp_from_sensors(&[s("/amdcpu/0/temperature/2", "Core (Tctl/Tdie)", "Temperature", 81.5)]), Some(81.5));
    assert_eq!(cpu_temp_from_sensors(&[s("/amdcpu/0/temperature/3", "CCD1", "Temperature", 60.0), s("/amdcpu/0/temperature/4", "CCD2", "Temperature", 66.0)]), Some(66.0));
}
#[test] fn picks_gpu_core() {
    let rows = [s("/gpu-nvidia/0/temperature/0", "GPU Core", "Temperature", 68.0), s("/gpu-nvidia/0/temperature/2", "GPU Hot Spot", "Temperature", 80.0)];
    assert_eq!(gpu_temp_from_sensors(&rows), Some(68.0));
}
#[test] fn thermal_zone_kelvin() {
    assert_eq!(thermal_zone_celsius(&[("\\_TZ.TZ00".into(), 300.15), ("\\_TZ.TZ01".into(), 345.15)]), Some(72.0));
    assert_eq!(thermal_zone_celsius(&[("z".into(), 0.0)]), None);
}
#[test] fn temps_absent_are_none() {
    assert_eq!(cpu_temp_from_sensors(&[]), None);
    assert_eq!(gpu_temp_from_sensors(&[]), None);
    assert_eq!(deci_celsius(0), None);
    assert_eq!(deci_celsius(655), Some(65.5));
}
```

- [ ] **Step 2: run** `cargo test -p si-probe --test temps` → compile errors.
- [ ] **Step 3: implement** `temps.rs` (spec §3 rules; `thermal_zone_celsius` rounds to 0.1 °C).
- [ ] **Step 4: Windows readers** — `win/lhm.rs`: `LhmReader { last_attempt: Option<Instant>, namespace: Option<&'static str> }` with `read(&mut self) -> Vec<LhmSensor>` (tries `ROOT\LibreHardwareMonitor` then `ROOT\OpenHardwareMonitor`; when both fail, waits 30 s before retrying); `win/gpu_perf.rs`: `gpu_temperature(luid: u64) -> Option<f32>` via `D3DKMTOpenAdapterFromLuid` → `D3DKMTQueryAdapterInfo(KMTQAITYPE_ADAPTERPERFDATA)` → `D3DKMTCloseAdapter`; `dxgi::enumerate_gpus` fills `GpuInfo.luid` (`(HighPart as u64) << 32 | LowPart`); `PdhSampler` adds `\Thermal Zone Information(*)\Temperature` (`PdhReading.thermal_zones`). `WindowsProbe::sample` sets `cpu_temp_c`/`cpu_temp_exact` (LHM, else thermal zone) and `gpu_temp_c` (NVML, else LHM, else D3DKMT of the primary GPU's LUID). `FixtureProbe::sample` adds a deterministic CPU temperature.
- [ ] **Step 5: verify** `cargo test -p si-probe` and the Windows cross-clippy; commit `feat(probe): CPU and GPU temperatures`.

### Task 2: Exact bottleneck report

**Files:** modify `crates/si-core/src/{frames,lib}.rs`, `crates/si-probe/src/presentmon.rs`, `crates/si-app/src/{worker,state}.rs`; create `crates/si-core/src/bottleneck.rs`; test `crates/si-core/tests/bottleneck.rs` (+ extend `tests/live.rs`).

**Interfaces (produces):**
- `frames::FrameSample { frame_ms: f64, cpu_busy_ms: Option<f64>, gpu_busy_ms: Option<f64> }`; `FrameTimeParser::feed(&mut self, line) -> Option<FrameSample>` (column aliases, case-insensitive: frame `MsBetweenPresents`/`FrameTime`; CPU `CPUBusy`/`MsCPUBusy`; GPU `GPUBusy`/`MsGPUBusy`/`MsGPUActive`); `feed_line` unchanged; `FrameWindow::push_sample`, `FrameWindow::samples() -> Vec<FrameSample>`
- `bottleneck::{EvidenceItem { label: String, value: String, share: Option<f32>, limiting: bool }, AnalysisSource { Frames(usize), Load(usize) }, BottleneckReport { verdict: Bottleneck, headline: String, source: AnalysisSource, evidence: Vec<EvidenceItem>, actions: Vec<String> }, analyze_bottleneck(samples: &[LiveSample], frames: &[FrameSample], display_max_hz: Option<u32>) -> BottleneckReport}`
- Worker `WorkerEvent::Frames { stats, frame_times, frames: Vec<FrameSample> }`; `AppState.frame_samples`, `AppState::bottleneck_report(&self) -> BottleneckReport`.

Headlines: InsufficientData "Collecting data…", VramLimited "Video memory (VRAM) is full", RamLimited "System RAM is full", CpuThrottled "The CPU is throttling", GpuThrottled "The GPU is throttling", GpuBound "The GPU is the bottleneck", CpuBound "The CPU is the bottleneck", FrameCapped "Frame rate is capped" (plus " at your monitor's {hz} Hz refresh rate" when within 3 %), Balanced "No single bottleneck".

- [ ] **Step 1: failing tests**

```rust
use si_core::{bottleneck::*, frames::*, live::*};
fn f(frame: f64, cpu: f64, gpu: f64) -> FrameSample { FrameSample { frame_ms: frame, cpu_busy_ms: Some(cpu), gpu_busy_ms: Some(gpu) } }
fn load(gpu: f32, core: f32) -> Vec<LiveSample> { vec![LiveSample { gpu_pct: Some(gpu), cpu_max_core_pct: core, cpu_total_pct: 40.0, ram_used_mb: 8000, ram_total_mb: 32000, ..Default::default() }; 10] }

#[test] fn parses_busy_columns() {
    let mut p = FrameTimeParser::new();
    p.feed("Application,FrameTime,CPUBusy,GPUBusy");
    assert_eq!(p.feed("g.exe,16.0,6.0,15.5"), Some(f(16.0, 6.0, 15.5)));
}
#[test] fn gpu_bound_from_frames() {
    let r = analyze_bottleneck(&load(99.0, 50.0), &vec![f(16.4, 7.0, 15.9); 60], Some(144));
    assert_eq!(r.verdict, Bottleneck::GpuBound);
    assert_eq!(r.source, AnalysisSource::Frames(60));
    assert_eq!(r.headline, "The GPU is the bottleneck");
    assert!(r.evidence.iter().any(|e| e.label == "GPU busy" && e.limiting && e.value == "15.9 ms of 16.4 ms per frame"));
}
#[test] fn cpu_bound_from_frames() {
    assert_eq!(analyze_bottleneck(&load(60.0, 97.0), &vec![f(10.0, 9.6, 5.0); 60], None).verdict, Bottleneck::CpuBound);
}
#[test] fn capped_at_refresh_rate() {
    let r = analyze_bottleneck(&load(40.0, 40.0), &vec![f(6.94, 3.0, 4.0); 60], Some(144));
    assert_eq!(r.verdict, Bottleneck::FrameCapped);
    assert_eq!(r.headline, "Frame rate is capped at your monitor's 144 Hz refresh rate");
}
#[test] fn vram_beats_frames() {
    let mut s = load(99.0, 50.0);
    for x in &mut s { x.vram_used_mb = Some(7900); x.vram_total_mb = Some(8000); }
    assert_eq!(analyze_bottleneck(&s, &vec![f(16.4, 7.0, 15.9); 60], None).verdict, Bottleneck::VramLimited);
}
#[test] fn frames_without_busy_use_load_mode() {
    let frames = vec![FrameSample { frame_ms: 16.0, cpu_busy_ms: None, gpu_busy_ms: None }; 60];
    let r = analyze_bottleneck(&load(99.0, 50.0), &frames, None);
    assert_eq!((r.verdict, r.source), (Bottleneck::GpuBound, AnalysisSource::Load(10)));
    assert!(!r.actions.is_empty());
}
#[test] fn temperatures_are_evidence() {
    let mut s = load(99.0, 50.0);
    for x in &mut s { x.cpu_temp_c = Some(96.0); x.gpu_temp_c = Some(70.0); }
    let r = analyze_bottleneck(&s, &[], None);
    assert!(r.evidence.iter().any(|e| e.label == "CPU temperature" && e.value == "96 °C (approx.)" && e.limiting));
}
```

- [ ] **Step 2–4:** run (fail) → implement `FrameSample` parsing, `bottleneck.rs`, PresentMon sending `FrameSample`, worker/state wiring → run `cargo test -p si-core` (pass).
- [ ] **Step 5:** commit `feat: exact per-frame bottleneck analysis`.

### Task 3: One-click driver installs through Windows Update

**Files:** modify `crates/si-core/src/drivers.rs`, `crates/si-probe/src/{lib,win/wua}.rs`, `crates/si-probe/Cargo.toml`, `crates/si-app/src/{main,worker,state}.rs`, `crates/si-app/src/pages/drivers.rs`, `crates/si-app/src/widgets/format.rs`; create `crates/si-probe/src/install.rs`, `crates/si-probe/src/win/{elevate,restore}.rs`; tests `crates/si-core/tests/drivers.rs`, `crates/si-probe/tests/install.rs`, `crates/si-app/tests/drivers_page.rs`.

**Interfaces (produces):**
- `AvailableDriverUpdate.update_id: String`; `DriverRow.install_id: Option<String>` (serde default)
- `install::InstallReport { installed: Vec<String>, failed: Vec<String>, reboot_required: bool, restore_point_created: bool }` (serde, Default)
- `install::is_update_id(&str) -> bool`; `install::helper_args(result: &Path, ids: &[String]) -> Vec<String>`; `install::parse_helper_args(args: &[String]) -> Option<(PathBuf, Vec<String>)>`
- `si_probe::install_driver_updates_elevated(ids: &[String]) -> Result<InstallReport, ProbeError>` (Windows: UAC relaunch; other OS: error) and `si_probe::install_driver_updates_here(ids: &[String]) -> InstallReport` (runs in the elevated helper)
- `WorkerCommand::InstallDrivers { update_ids: Vec<String> }`, `WorkerEvent::DriversInstalled(InstallReport)`; `AppState { drivers_installing: bool, install_confirm: Option<Vec<String>>, reboot_required: bool }`
- Pill labels via `format::driver_status_label`: WindowsUpdate → "Install", Catalog → "Download".

- [ ] **Step 1: failing tests**

```rust
// si-core tests/drivers.rs
#[test] fn windows_update_rows_are_installable() {
    let wu = [AvailableDriverUpdate { hardware_id: "pci\\ven_8086&dev_46a8".into(), update_id: "0b5b8d3c-1a2b-4c5d-8e9f-001122334455".into(), ..Default::default() }];
    let rows = evaluate_drivers(&fixtures::struggling_laptop(), &DriverCatalog::empty(), &wu);
    let iris = rows.iter().find(|r| r.driver.device_name == "Intel(R) Iris(R) Xe Graphics").unwrap();
    assert_eq!(iris.install_id.as_deref(), Some("0b5b8d3c-1a2b-4c5d-8e9f-001122334455"));
}
// si-probe tests/install.rs
#[test] fn helper_args_round_trip() {
    let ids = vec!["0b5b8d3c-1a2b-4c5d-8e9f-001122334455".to_string()];
    let args = helper_args(Path::new("C:\\Temp\\r.json"), &ids);
    assert_eq!(parse_helper_args(&args), Some((PathBuf::from("C:\\Temp\\r.json"), ids)));
}
#[test] fn install_args_reject_non_guid() {
    for bad in ["; calc", "0b5b8d3c-1a2b-4c5d-8e9f-00112233445", "../../x", ""] {
        assert!(!is_update_id(bad), "{bad}");
        assert_eq!(parse_helper_args(&["--install-drivers".into(), "r.json".into(), bad.into()]), None);
    }
    assert_eq!(parse_helper_args(&["--install-drivers".into(), "r.json".into()]), None);
}
// si-app tests/drivers_page.rs
#[test] fn install_asks_for_confirmation_then_sends() { /* WU-offered row → "Install" pill → dialog → "Install now" → Send(InstallDrivers{..}), drivers_installing */ }
#[test] fn install_failure_clears_state() { /* drivers_installing=true; apply Error → false; DriversInstalled{reboot_required:true} → reboot banner text "Restart your PC to finish installing drivers." */ }
```

- [ ] **Step 2–4:** run (fail) → implement → run (pass). Windows code: `win/wua.rs` adds `update_id` to search results and `install(ids) -> InstallReport` (search, keep matching IDs, `AcceptEula`, `CreateUpdateDownloader` + `SetUpdates` + `Download`, `CreateUpdateInstaller` + `SetUpdates` + `Install`, per-update `GetUpdateResult(i).ResultCode() == orcSucceeded`); `win/restore.rs` (`SRSetRestorePointW` BEGIN/END, `DEVICE_DRIVER_INSTALL`); `win/elevate.rs` (`ShellExecuteExW` verb `runas`, `SEE_MASK_NOCLOSEPROCESS`, `WaitForSingleObject(INFINITE)`, read the JSON report; `ERROR_CANCELLED` ⇒ `ProbeError::Windows("Installation cancelled")`). `main.rs` handles `--install-drivers` before creating any window.
- [ ] **Step 5:** cross-clippy; commit `feat: one-click driver installs through Windows Update`.

### Task 4: Professional redesign

**Files:** modify `crates/si-app/src/{theme,app,nav,settings,state}.rs`, all `pages/*.rs`, `widgets/*.rs`; tests in `crates/si-app/tests/*` updated to the new copy.

Decisions: palettes (dark: bg `#111318`, sidebar `#16191F`, card `#1B1F27`, raised `#232833`, border `#2C3240`, text `#E6E8EC`, muted `#9AA3B2`, accent `#3B82F6`, accent strong `#2563EB`, success `#22A06B`, warning `#D9822B`, danger `#E5484D`; light: bg `#F5F6F8`, sidebar `#ECEEF2`, card `#FFFFFF`, raised `#F0F2F5`, border `#DDE1E7`, text `#1B1F24`, muted `#5B6472`, accent `#2563EB`, accent strong `#1D4ED8`, success `#1A7F52`, warning `#B5651D`, danger `#C9372C`); card radius 8, pill radius 6; `ThemeChoice { #[default] System, Light, Dark }`; Segoe UI (`C:\Windows\Fonts\segoeui.ttf`) as the first proportional font when present; page titles Overview / Issues / Performance / Drivers / Games / Settings; sidebar 200 px with icon + label; Overview layout per spec §3; Performance page = Bottleneck analysis card (headline, source line, evidence rows with share bars, actions) + charts + temperatures + processes.

- [ ] **Step 1:** update tests to the new copy (Overview tiles `Health`, `Bottleneck`, `CPU`, `GPU`; table `Performance issues`; `Install all (n)`; Settings `Theme`), run (fail).
- [ ] **Step 2:** implement; run `cargo test --workspace` (pass).
- [ ] **Step 3:** visual check under Xvfb (dark 1440×900, light 1100×700); commit `feat(app): professional redesign`.

### Task 5: Docs and verification

- [ ] README: temperatures (LibreHardwareMonitor for exact CPU temps), bottleneck analysis, driver installs (UAC, restore point), new screenshots.
- [ ] Push; CI green on Windows and Ubuntu; fast-forward `main`.
