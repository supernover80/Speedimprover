# SpeedImprover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Windows desktop app in Rust that scans the PC, ranks what is hurting FPS, flags outdated drivers and recommends per-game settings, with a dark dashboard UI modeled on the supplied mockup.

**Architecture:** Three-crate Cargo workspace. `si-core` is pure logic (model, FPS rules, bottleneck classifier, driver status, hardware tiers, game parsing, recommendations) and tests on any OS. `si-probe` collects data (sysinfo everywhere; WMI/registry/DXGI/PDH/NVML/Windows Update behind `cfg(windows)`; a fixture probe for dev). `si-app` is an eframe/egui binary whose worker thread owns the probe and talks to the UI over channels.

**Tech Stack:** Rust 2024, eframe/egui 0.36, egui_plot, egui-phosphor, sysinfo, wmi, winreg, windows-rs, nvml-wrapper, reqwest (blocking), keyvalues-parser, serde/serde_json/toml, chrono, crossbeam-channel, egui_kittest.

**Spec:** `docs/superpowers/specs/2026-11-14-speedimprover-design.md`

## Global Constraints

- Rust edition `2024`, `rust-version = "1.85"`, workspace `resolver = "3"`.
- Dependency versions (verified to resolve and build together on 2026-11-14): `eframe = "0.36"`, `egui = "0.36"`, `egui_plot = "0.37"`, `egui-phosphor = "0.14.0"`, `egui_kittest = "0.36"` (dev), `sysinfo = "0.40.3"`, `wmi = "0.18.4"`, `winreg = "0.56.0"`, `windows = "0.62.2"`, `nvml-wrapper = "0.13.0"`, `reqwest = { version = "0.13.5", default-features = false, features = ["blocking", "json", "rustls"] }`, `keyvalues-parser = "0.2.4"`, `serde = { version = "1.0.229", features = ["derive"] }`, `serde_json = "1.0.154"`, `toml = "1.2.1"`, `chrono = "0.4.45"`, `thiserror = "2.0.21"`, `anyhow = "1.0.104"`, `tracing = "0.1.44"`, `tracing-subscriber = "0.3.23"`, `crossbeam-channel = "0.5.17"`, `open = "5.4.5"`, `directories = "6.0.0"`, `winresource = "0.1.34"` (build).
- Stay on the egui 0.36 family until `egui-phosphor` releases support for egui 0.37 (0.14.0 requires `egui ^0.36`).
- `windows` crate features: `Win32_Foundation`, `Win32_Graphics_Dxgi`, `Win32_Graphics_Dxgi_Common`, `Win32_Graphics_Direct3D`, `Win32_Graphics_Direct3D12`, `Win32_Graphics_Gdi`, `Win32_System_Performance`, `Win32_System_Power`, `Win32_System_Com`, `Win32_System_Ole`, `Win32_System_Variant`, `Win32_System_UpdateAgent`, `Win32_System_SystemServices`.
- `si-core` depends on no OS-specific crate (no `windows`, `wmi`, `winreg`, `sysinfo`, `reqwest`, `nvml-wrapper`).
- All Windows-only code is behind `#[cfg(windows)]`. `cargo test --workspace` passes on both Linux and Windows.
- The Windows module in `si-probe` is named `win` (a module named `windows` would clash with the `windows` crate).
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass at every commit.
- No `unwrap()`/`expect()` on OS or file data outside tests; collectors record a `ScanWarning` instead.
- The app is read-only towards the system: it writes only its own config/data files and never edits the registry, drivers or game files.
- The only network calls are the driver catalog URL (10 s timeout) and the Windows Update Agent API.
- User-facing copy that the spec fixes (finding titles, bottleneck explanations, pill labels) is used verbatim.
- Commit after every task with a Conventional Commit message (`feat(core): …`, `feat(probe): …`, `feat(app): …`, `build: …`).

## Review Focus

1. **Partial data** — WMI broken, non-admin user, no NVIDIA driver, AMD/Intel GPU: the scan finishes, missing fields are `None`/`0`, no rule fires on unknowns, nothing panics. Pinned by `unknowns_produce_no_findings` (Task 2), `missing_metrics_never_match` (Task 3), `scan_survives_failing_collectors` (Task 9).
2. **Hybrid laptops and virtual adapters** — iGPU + dGPU, "Microsoft Basic Render Driver", Parsec/RDP adapters: tiers and rules use the strongest real discrete GPU. Pinned by `primary_gpu_prefers_discrete_with_most_vram` (Task 1), `hybrid_laptop_uses_discrete` (Task 5), `skips_microsoft_and_software_adapters` (Task 9).
3. **Fractional refresh rates** — 59.94/60 Hz, 143.9/144 Hz report as 59/60, 143/144: never flagged. Pinned by `fractional_refresh_not_flagged` (Task 2).
4. **Malformed or partial game libraries** — removed library drives, half-installed games, garbage manifests: skip the bad entry, keep the rest. Pinned by `malformed_inputs_are_skipped` (Task 6) and `missing_library_folder_is_skipped` (Task 10).
5. **Non-English Windows** — localized counter names and power plan names: PDH uses `PdhAddEnglishCounterW`, power plans are identified by GUID. Pinned by `power_plan_by_guid` (Task 10) and `windows_sample_has_cpu_perf` (Task 11, Windows CI).

---

## File Structure

```
Cargo.toml                         workspace
.github/workflows/ci.yml           fmt, clippy, test on ubuntu + windows; release on tag
data/driver_catalog.json           latest driver versions (fetched at runtime too)
data/gpu_tiers.toml  data/cpu_tiers.toml
data/games/*.toml                  one game profile per file
crates/si-core/src/
  lib.rs  error.rs  model.rs  fixtures.rs  fixtures/*.json
  findings.rs  rules.rs            Finding types, health score, FPS rules
  live.rs  frames.rs               bottleneck classifier, PresentMon CSV + FPS stats
  drivers.rs  tier.rs
  games/{mod,library,profile,recommend}.rs
crates/si-probe/src/
  lib.rs                           SystemProbe trait, ScanStep/Warning/Result, ProbeError, default_probe()
  generic.rs  fixture.rs           sysinfo probe (all OS), fixture probe
  mapping.rs  winsettings.rs  gpu_counters.rs   pure helpers for Windows data (tested on all OS)
  catalog.rs  presentmon.rs
  win/{mod,wmi,dxgi,registry,display,power,games,pdh,nvml,wua}.rs       cfg(windows)
crates/si-app/
  build.rs  assets/{demo.json,icon.ico}
  src/main.rs  lib.rs  app.rs  state.rs  theme.rs  settings.rs  worker.rs  nav.rs  testing.rs
  src/widgets/{mod,card,pill,donut,chart,toasts,format}.rs
  src/pages/{mod,dashboard,analyzer,live,drivers,games,settings}.rs
```

---

## Phase 1 — Core logic (`si-core`, any OS)

### Task 1: Workspace, CI and data model

**Files:**
- Create: `Cargo.toml`, `.gitignore` (`/target`), `.github/workflows/ci.yml`
- Create: `crates/si-core/Cargo.toml`, `crates/si-core/src/lib.rs`, `crates/si-core/src/model.rs`, `crates/si-core/src/fixtures.rs`
- Create: `crates/si-core/src/fixtures/healthy_desktop.json`, `crates/si-core/src/fixtures/struggling_laptop.json`
- Test: `crates/si-core/tests/model.rs`

**Interfaces:**
- Produces (`si_core::model`; every type derives `Debug, Clone, PartialEq, Default, Serialize, Deserialize`; every struct has `#[serde(default)]`; enums use `#[serde(rename_all = "snake_case")]`):
  - `SystemSnapshot { taken_at_unix: i64, machine: MachineInfo, cpu: CpuInfo, gpus: Vec<GpuInfo>, memory: MemoryInfo, disks: Vec<DiskInfo>, displays: Vec<DisplayInfo>, os: OsSettings, processes: Vec<ProcessUsage>, drivers: Vec<DriverInfo> }`
  - `MachineInfo { hostname: String, os_name: String, os_build: u32, is_laptop: Option<bool>, on_battery: Option<bool> }`
  - `CpuInfo { name: String, physical_cores: u32, logical_cores: u32, base_mhz: u32 }`
  - `Vendor { Nvidia, Amd, Intel, #[default] Other }`
  - `GpuInfo { name: String, vendor: Vendor, vram_mb: u64, is_integrated: bool, driver_version: Option<String>, driver_date: Option<String> }` (dates are `YYYY-MM-DD`)
  - `MemoryInfo { total_mb: u64, used_mb: u64, modules: Vec<MemoryModule> }`
  - `MemoryGen { Ddr4, Ddr5, #[default] Other }`; `MemoryModule { capacity_mb: u64, rated_mhz: u32, configured_mhz: u32, generation: MemoryGen }`
  - `DiskMedia { Nvme, Ssd, Hdd, #[default] Unknown }`; `DiskInfo { mount: String, media: DiskMedia, total_gb: f64, free_gb: f64 }`
  - `DisplayInfo { name: String, width: u32, height: u32, current_hz: u32, max_hz: u32 }`
  - `PowerPlan { Balanced, HighPerformance, PowerSaver, Ultimate, Custom, #[default] Unknown }`
  - `OsSettings { game_mode: Option<bool>, background_recording: Option<bool>, hags: Option<bool>, memory_integrity: Option<bool>, power_plan: PowerPlan, startup_apps: Option<u32> }`
  - `ProcessUsage { pid: u32, name: String, cpu_pct: f32, mem_mb: u64, gpu_pct: Option<f32> }`
  - `DriverInfo { device_name: String, class: String, manufacturer: String, version: String, date: Option<String>, hardware_ids: Vec<String> }`
  - `Store { #[default] Steam, Epic }`; `InstalledGame { store: Store, id: String, name: String, install_dir: String, size_bytes: Option<u64> }`
  - `impl SystemSnapshot { fn primary_gpu(&self) -> Option<&GpuInfo>; fn has_discrete_gpu(&self) -> bool; fn system_disk(&self) -> Option<&DiskInfo>; fn age_days(&self, date: &str) -> Option<i64> }`
  - `impl InstalledGame { fn key(&self) -> String }` → `"steam:730"`, `"epic:Fortnite"`
  - `si_core::fixtures::{healthy_desktop() -> SystemSnapshot, struggling_laptop() -> SystemSnapshot}` (always compiled; `include_str!` + `serde_json`)

- [ ] **Step 1: Create the workspace and CI**

`Cargo.toml` lists `crates/si-core` as the only member for now and holds `[workspace.package]` (edition, rust-version, version `0.1.0`) and `[workspace.dependencies]` for the shared crates in Global Constraints. `ci.yml` runs on push and pull_request with a matrix `os: [ubuntu-latest, windows-latest]`: `actions/checkout@v4`, `dtolnay/rust-toolchain@stable` (components `clippy, rustfmt`), `Swatinem/rust-cache@v2`, then `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.

- [ ] **Step 2: Write the fixtures**

`taken_at_unix = 1793491200` (2026-11-01) in both.

| Field | `healthy_desktop.json` | `struggling_laptop.json` |
|---|---|---|
| machine | `DESKTOP-GAMER`, `Windows 11`, build 26100, laptop false, battery false | `LAPTOP-STUDENT`, `Windows 11`, build 22631, laptop true, battery true |
| cpu | `AMD Ryzen 7 7800X3D 8-Core Processor`, 8/16, 4200 | `12th Gen Intel(R) Core(TM) i5-1235U`, 10/12, 1300 |
| gpus | `NVIDIA GeForce RTX 4070 SUPER`, nvidia, 12282 MB, discrete, `32.0.15.6614`, `2026-10-01` | `Intel(R) Iris(R) Xe Graphics`, intel, 128 MB, integrated, `31.0.101.4502`, `2024-06-01` |
| memory | total 32768, used 9000; 2 × {16384, 6000, 6000, ddr5} | total 7900, used 7000; 1 × {8192, rated 3200, configured 2666, ddr4} |
| disks | `C:\` nvme 931.5 / 412.0 free | `C:\` ssd 237.0 / 9.5 free |
| displays | `\\.\DISPLAY1` 2560×1440, 165/165 | `\\.\DISPLAY1` 1920×1080, 60/144 |
| os | game_mode true, recording false, hags true, memory_integrity false, balanced, startup 6 | game_mode false, recording true, hags null, memory_integrity true, power_saver, startup 22 |
| processes | steam.exe 1.2 %/350 MB; chrome.exe 3.0 %/900 MB | chrome.exe 18.5/2200; OneDrive.exe 11.0/300; dwm.exe 25.0/150; Teams.exe 4.0/1600 |
| drivers | GPU driver (class `Display`, `NVIDIA`, same version/date, `PCI\VEN_10DE&DEV_2783`); `Realtek PCIe 2.5GbE Family Controller` (`Net`, `Realtek`, `10.68.815.2023`, `2026-08-15`) | GPU driver (`Display`, `Intel Corporation`, same version/date, `PCI\VEN_8086&DEV_46A8`); `Intel(R) Wi-Fi 6 AX201 160MHz` (`Net`, `Intel Corporation`, `22.200.0.6`, `2023-03-01`, `PCI\VEN_8086&DEV_A0F0`) |

- [ ] **Step 3: Write the failing tests** in `crates/si-core/tests/model.rs`

```rust
use si_core::{fixtures, model::*};

#[test]
fn fixtures_round_trip() {
    for s in [fixtures::healthy_desktop(), fixtures::struggling_laptop()] {
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<SystemSnapshot>(&json).unwrap(), s);
    }
}

#[test]
fn partial_json_uses_defaults() {
    let s: SystemSnapshot = serde_json::from_str(r#"{"cpu":{"name":"X"}}"#).unwrap();
    assert_eq!(s.cpu.name, "X");
    assert_eq!(s.os.power_plan, PowerPlan::Unknown);
}

#[test]
fn primary_gpu_prefers_discrete_with_most_vram() {
    let mut s = fixtures::struggling_laptop();
    s.gpus.push(GpuInfo { name: "NVIDIA GeForce RTX 4060 Laptop GPU".into(), vendor: Vendor::Nvidia, vram_mb: 8188, ..Default::default() });
    assert_eq!(s.primary_gpu().unwrap().vram_mb, 8188);
    assert!(s.has_discrete_gpu());
    assert!(!fixtures::struggling_laptop().has_discrete_gpu());
}

#[test]
fn age_days_counts_from_snapshot_time() {
    let s = fixtures::healthy_desktop();
    assert_eq!(s.age_days("2026-10-01"), Some(31));
    assert_eq!(s.age_days("not a date"), None);
}

#[test]
fn system_disk_and_game_key() {
    assert_eq!(fixtures::healthy_desktop().system_disk().unwrap().media, DiskMedia::Nvme);
    let g = InstalledGame { store: Store::Epic, id: "Fortnite".into(), ..Default::default() };
    assert_eq!(g.key(), "epic:Fortnite");
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test -p si-core --test model`
Expected: compile errors (types not defined).

- [ ] **Step 5: Implement the model and fixtures**

`system_disk` matches a mount starting with `C:` case-insensitively. `age_days` parses with `chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")` and subtracts from the date of `taken_at_unix`.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p si-core --test model` → all 5 pass. Then `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` → clean.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml .gitignore .github crates/si-core
git commit -m "feat(core): workspace, CI and system snapshot model"
```

### Task 2: FPS rules and health score

**Files:**
- Create: `crates/si-core/src/findings.rs`, `crates/si-core/src/rules.rs`
- Test: `crates/si-core/tests/rules.rs`

**Interfaces:**
- Consumes: Task 1 model and fixtures.
- Produces:
  - `findings::Severity { Low, Medium, High }` (derive `Ord`, order as written)
  - `findings::Category { Display, Power, Memory, Os, Gpu, Storage, Processes, Startup }`
  - `findings::Fix { OpenUri(String), Manual(String), ShowDrivers }`
  - `findings::Finding { id: &'static str, severity: Severity, category: Category, title: String, detail: String, fix: Fix }` (derive `Debug, Clone, PartialEq, Serialize`)
  - `findings::health_score(findings: &[Finding]) -> u8`
  - `rules::analyze(snapshot: &SystemSnapshot, games: &[InstalledGame]) -> Vec<Finding>`

- [ ] **Step 1: Write the failing tests**

```rust
use si_core::{findings::*, fixtures, model::*, rules::analyze};

fn ids(s: &SystemSnapshot, g: &[InstalledGame]) -> Vec<&'static str> { analyze(s, g).iter().map(|f| f.id).collect() }
fn find(s: &SystemSnapshot, id: &str) -> Option<Finding> { analyze(s, &[]).into_iter().find(|f| f.id == id) }

#[test]
fn healthy_fixture_has_no_findings() {
    let f = analyze(&fixtures::healthy_desktop(), &[]);
    assert!(f.is_empty(), "{f:#?}");
    assert_eq!(health_score(&f), 100);
}

#[test]
fn struggling_fixture_flags_expected_rules() {
    let mut got = ids(&fixtures::struggling_laptop(), &[]);
    got.sort();
    assert_eq!(got, ["display.refresh_below_max", "gpu.driver_age", "memory.below_rated", "memory.low_total",
        "memory.pressure", "memory.single_channel", "os.background_recording_on", "os.game_mode_off",
        "os.memory_integrity_on", "power.on_battery", "power.plan_saver", "process.hogs",
        "startup.too_many", "storage.low_free"]);
    assert_eq!(health_score(&analyze(&fixtures::struggling_laptop(), &[])), 0);
}

#[test]
fn findings_sorted_by_severity_then_id() {
    let f = analyze(&fixtures::struggling_laptop(), &[]);
    assert!(f.windows(2).all(|w| w[0].severity > w[1].severity || (w[0].severity == w[1].severity && w[0].id <= w[1].id)));
}

#[test]
fn unknowns_produce_no_findings() {
    assert!(analyze(&SystemSnapshot::default(), &[]).is_empty());
}

#[test]
fn fractional_refresh_not_flagged() {
    for (cur, max, flagged) in [(59, 60, false), (143, 144, false), (60, 75, true), (60, 144, true)] {
        let mut s = fixtures::healthy_desktop();
        (s.displays[0].current_hz, s.displays[0].max_hz) = (cur, max);
        assert_eq!(ids(&s, &[]).contains(&"display.refresh_below_max"), flagged, "{cur}/{max}");
    }
}

#[test]
fn single_channel_severity_depends_on_discrete_gpu() {
    let mut s = fixtures::healthy_desktop();
    s.memory.modules.truncate(1);
    assert_eq!(find(&s, "memory.single_channel").unwrap().severity, Severity::Medium);
    assert_eq!(find(&fixtures::struggling_laptop(), "memory.single_channel").unwrap().severity, Severity::High);
}

#[test]
fn memory_pressure_thresholds() {
    let mut s = fixtures::healthy_desktop();
    s.memory.used_mb = 26_300; // 0.80
    assert_eq!(find(&s, "memory.pressure").unwrap().severity, Severity::Medium);
    s.memory.used_mb = 28_200; // 0.86
    assert_eq!(find(&s, "memory.pressure").unwrap().severity, Severity::High);
}

#[test]
fn jedec_default_only_on_desktops() {
    let mut s = fixtures::healthy_desktop();
    for m in &mut s.memory.modules { (m.rated_mhz, m.configured_mhz) = (4800, 4800); }
    assert_eq!(find(&s, "memory.jedec_default").unwrap().severity, Severity::Low);
    s.machine.is_laptop = Some(true);
    assert!(find(&s, "memory.jedec_default").is_none());
}

#[test]
fn process_hogs_lists_top_three_by_cpu() {
    let d = find(&fixtures::struggling_laptop(), "process.hogs").unwrap().detail;
    for name in ["chrome.exe", "OneDrive.exe", "Teams.exe"] { assert!(d.contains(name), "{d}"); }
    assert!(!d.contains("dwm.exe"));
}

#[test]
fn games_on_hdd_uses_longest_mount() {
    let mut s = fixtures::healthy_desktop();
    s.disks.push(DiskInfo { mount: "D:\\".into(), media: DiskMedia::Hdd, total_gb: 2000.0, free_gb: 900.0 });
    let on_d = InstalledGame { name: "Foo".into(), install_dir: "d:\\SteamLibrary\\steamapps\\common\\Foo".into(), ..Default::default() };
    let on_c = InstalledGame { name: "Bar".into(), install_dir: "C:\\Games\\Bar".into(), ..Default::default() };
    assert!(ids(&s, &[on_d]).contains(&"storage.games_on_hdd"));
    assert!(!ids(&s, &[on_c]).contains(&"storage.games_on_hdd"));
}

#[test]
fn fixes_and_titles_match_spec() {
    let s = fixtures::struggling_laptop();
    let gm = find(&s, "os.game_mode_off").unwrap();
    assert_eq!(gm.title, "Game Mode is off");
    assert_eq!(gm.fix, Fix::OpenUri("ms-settings:gaming-gamemode".into()));
    assert_eq!(find(&s, "gpu.driver_age").unwrap().fix, Fix::ShowDrivers);
}

#[test]
fn health_score_mixed() {
    let mut s = fixtures::healthy_desktop();
    s.os.game_mode = Some(false); // Medium
    s.os.hags = Some(false);      // Low
    assert_eq!(health_score(&analyze(&s, &[])), 89);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p si-core --test rules` → compile errors.

- [ ] **Step 3: Implement `findings.rs` and `rules.rs`**

One private function per rule (`fn refresh_below_max(s: &SystemSnapshot) -> Option<Finding>` etc.; `memory.single_channel` and `process.hogs` return at most one finding). Thresholds, titles, fixes and the process ignore list come from spec §5. Details include the concrete numbers (e.g. "Running at 60 Hz, supports 144 Hz", "RAM at 2666 MT/s, rated 3200 MT/s"). A total of `0` RAM or disk size skips the rule. `analyze` collects all, sorts by severity descending then id ascending.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p si-core --test rules` → 12 pass.

- [ ] **Step 5: Commit**

```bash
git add crates/si-core
git commit -m "feat(core): FPS rules engine and health score"
```

### Task 3: Live bottleneck classifier and frame statistics

**Files:**
- Create: `crates/si-core/src/live.rs`, `crates/si-core/src/frames.rs`
- Test: `crates/si-core/tests/live.rs`

**Interfaces:**
- Consumes: `model::ProcessUsage`.
- Produces:
  - `live::LiveSample { t_ms: u64, cpu_total_pct: f32, cpu_max_core_pct: f32, cpu_perf_pct: Option<f32>, gpu_pct: Option<f32>, vram_used_mb: Option<u64>, vram_total_mb: Option<u64>, ram_used_mb: u64, ram_total_mb: u64, gpu_temp_c: Option<f32>, gpu_throttling: Option<bool>, top_processes: Vec<ProcessUsage> }` (derive `Debug, Clone, PartialEq, Default, Serialize, Deserialize`)
  - `live::Bottleneck { InsufficientData, VramLimited, RamLimited, CpuThrottled, GpuThrottled, GpuBound, CpuBound, FrameCapped, Balanced }` with `fn explanation(&self) -> &'static str`
  - `live::classify(samples: &[LiveSample]) -> Bottleneck`
  - `live::LiveHistory { fn new(capacity: usize) -> Self; fn push(&mut self, s: LiveSample); fn latest(&self) -> Option<&LiveSample>; fn samples(&self) -> impl Iterator<Item = &LiveSample>; fn len(&self) -> usize; fn verdict(&self) -> Bottleneck }`
  - `frames::FrameTimeParser { fn new() -> Self; fn feed_line(&mut self, line: &str) -> Option<f64> }`
  - `frames::FrameStats { avg_fps: f32, low_1pct_fps: f32, frames: usize }`; `frames::frame_stats(frame_times_ms: &[f64]) -> Option<FrameStats>`
  - `frames::FrameWindow { fn new(window_ms: f64) -> Self; fn push(&mut self, ms: f64); fn as_slice(&self) -> Vec<f64> }`

`explanation()` copy (exact):

| Variant | Text |
|---|---|
| InsufficientData | Collecting data… |
| VramLimited | Video memory is full — lower texture quality or resolution. |
| RamLimited | System RAM is nearly full — close background apps. |
| CpuThrottled | CPU is slowing down (heat or power limit) — check cooling and power plan. |
| GpuThrottled | GPU is throttling from heat — check cooling and fan curve. |
| GpuBound | GPU is the limit — lower resolution, enable upscaling or reduce GPU-heavy settings. |
| CpuBound | CPU is the limit — lower CPU-heavy settings (view distance, crowds, physics) and close background apps. |
| FrameCapped | Neither CPU nor GPU is maxed — a frame cap or V-Sync is probably limiting FPS. |
| Balanced | No single bottleneck — the system is well balanced. |

- [ ] **Step 1: Write the failing tests**

```rust
use si_core::{frames::*, live::*};

fn s(gpu: Option<f32>, core: f32, total: f32) -> LiveSample {
    LiveSample { cpu_total_pct: total, cpu_max_core_pct: core, gpu_pct: gpu, ram_used_mb: 4000, ram_total_mb: 16000, ..Default::default() }
}
fn n(x: LiveSample, k: usize) -> Vec<LiveSample> { vec![x; k] }

#[test] fn insufficient_with_four_samples() { assert_eq!(classify(&n(s(Some(99.0), 50.0, 50.0), 4)), Bottleneck::InsufficientData); }
#[test] fn gpu_bound() { assert_eq!(classify(&n(s(Some(98.0), 60.0, 40.0), 10)), Bottleneck::GpuBound); }
#[test] fn cpu_bound_by_single_core() { assert_eq!(classify(&n(s(Some(60.0), 97.0, 40.0), 10)), Bottleneck::CpuBound); }
#[test] fn frame_capped() { assert_eq!(classify(&n(s(Some(50.0), 40.0, 20.0), 10)), Bottleneck::FrameCapped); }
#[test] fn balanced() { assert_eq!(classify(&n(s(Some(88.0), 70.0, 50.0), 10)), Bottleneck::Balanced); }

#[test] fn vram_beats_gpu_bound() {
    let x = LiveSample { vram_used_mb: Some(7900), vram_total_mb: Some(8000), ..s(Some(99.0), 50.0, 50.0) };
    assert_eq!(classify(&n(x, 10)), Bottleneck::VramLimited);
}
#[test] fn cpu_throttle() {
    let x = LiveSample { cpu_perf_pct: Some(70.0), ..s(Some(60.0), 80.0, 60.0) };
    assert_eq!(classify(&n(x, 10)), Bottleneck::CpuThrottled);
}
#[test] fn missing_metrics_never_match() { assert_eq!(classify(&n(s(None, 40.0, 20.0), 10)), Bottleneck::Balanced); }
#[test] fn uses_only_last_ten() {
    let mut v = n(s(Some(99.0), 50.0, 50.0), 10);
    v.extend(n(s(Some(50.0), 40.0, 20.0), 10));
    assert_eq!(classify(&v), Bottleneck::FrameCapped);
}
#[test] fn history_caps_capacity() {
    let mut h = LiveHistory::new(120);
    for i in 0..130 { h.push(LiveSample { t_ms: i * 1000, ..Default::default() }); }
    assert_eq!(h.len(), 120);
    assert_eq!(h.samples().next().unwrap().t_ms, 10_000);
}

#[test] fn parses_v1_and_v2_columns() {
    let mut p = FrameTimeParser::new();
    assert_eq!(p.feed_line("Application,ProcessID,MsBetweenPresents"), None);
    assert_eq!(p.feed_line("game.exe,42,16.6"), Some(16.6));
    let mut p = FrameTimeParser::new();
    p.feed_line("Application,FrameTime,ProcessID");
    assert_eq!(p.feed_line("game.exe,8.3,42"), Some(8.3));
    assert_eq!(p.feed_line("game.exe,NA,42"), None);
}
#[test] fn unknown_header_yields_nothing() {
    let mut p = FrameTimeParser::new();
    p.feed_line("A,B,C");
    assert_eq!(p.feed_line("1,2,3"), None);
}
#[test] fn frame_stats_avg_and_one_percent_low() {
    let mut v = vec![10.0; 98];
    v.extend([50.0, 50.0]);
    let st = frame_stats(&v).unwrap();
    assert!((st.avg_fps - 92.59).abs() < 0.01);
    assert_eq!(st.low_1pct_fps, 20.0);
    assert_eq!(frame_stats(&[]), None);
}
#[test] fn frame_window_keeps_five_seconds() {
    let mut w = FrameWindow::new(5000.0);
    for _ in 0..600 { w.push(10.0); }
    assert_eq!(w.as_slice().len(), 500);
}
```

- [ ] **Step 2: Run tests to verify they fail** — `cargo test -p si-core --test live` → compile errors.

- [ ] **Step 3: Implement `live.rs` and `frames.rs`**

`classify` averages the last 10 samples and applies spec §6 in priority order; an average over `Option` fields uses only `Some` values and is `None` if there are none. 99th percentile = nearest rank: `sorted[ceil(0.99 * n) - 1]`.

- [ ] **Step 4: Run tests to verify they pass** — `cargo test -p si-core --test live` → 15 pass.

- [ ] **Step 5: Commit** — `git add crates/si-core && git commit -m "feat(core): bottleneck classifier and frame statistics"`

### Task 4: Driver status

**Files:**
- Create: `crates/si-core/src/error.rs`, `crates/si-core/src/drivers.rs`, `data/driver_catalog.json`
- Test: `crates/si-core/tests/drivers.rs`

**Interfaces:**
- Consumes: Task 1 model.
- Produces:
  - `error::CoreError { Json(#[from] serde_json::Error), Toml(#[from] toml::de::Error) }` (thiserror)
  - `drivers::compare_versions(a: &str, b: &str) -> Option<std::cmp::Ordering>`
  - `drivers::nvidia_display_version(windows_version: &str) -> Option<String>`
  - `drivers::DriverVendor { Nvidia, Amd, Intel }` (serde lowercase); `drivers::driver_vendor(manufacturer: &str) -> Option<DriverVendor>`
  - `drivers::CatalogEntry { vendor: DriverVendor, class: String, latest_version: String, download_url: String }`
  - `drivers::DriverCatalog { schema: u32, updated: String, entries: Vec<CatalogEntry> }` with `fn parse(json: &str) -> Result<Self, CoreError>`, `fn bundled() -> Self`, `fn empty() -> Self`
  - `drivers::AvailableDriverUpdate { title: String, hardware_id: String, driver_class: String, driver_model: String, ver_date: Option<String> }`
  - `drivers::UpdateSource { WindowsUpdate, Catalog }`
  - `drivers::DriverStatus { UpdateAvailable { source: UpdateSource, latest: Option<String>, url: String }, PossiblyOutdated { age_days: i64 }, NoUpdateFound }`
  - `drivers::DriverRow { driver: DriverInfo, status: DriverStatus }`
  - `drivers::is_shown_class(d: &DriverInfo) -> bool`
  - `drivers::evaluate_drivers(s: &SystemSnapshot, catalog: &DriverCatalog, wu: &[AvailableDriverUpdate]) -> Vec<DriverRow>`
  - All of the above derive `Debug, Clone, PartialEq, Serialize, Deserialize`; `AvailableDriverUpdate` also derives `Default`.

- [ ] **Step 1: Write the failing tests**

```rust
use si_core::{drivers::*, fixtures, model::DriverInfo};
use std::cmp::Ordering::*;

fn catalog(vendor: &str, class: &str, latest: &str) -> DriverCatalog {
    DriverCatalog::parse(&format!(r#"{{"schema":1,"updated":"2026-11-01","entries":[{{"vendor":"{vendor}","class":"{class}","latest_version":"{latest}","download_url":"https://example.test/dl"}}]}}"#)).unwrap()
}
fn status_of<'a>(rows: &'a [DriverRow], name: &str) -> &'a DriverStatus { &rows.iter().find(|r| r.driver.device_name == name).unwrap().status }

#[test] fn compares_dotted_versions() {
    assert_eq!(compare_versions("32.0.15.6094", "32.0.15.6100"), Some(Less));
    assert_eq!(compare_versions("1.2", "1.2.0"), Some(Equal));
    assert_eq!(compare_versions("abc", "1.0"), None);
    assert_eq!(compare_versions("", "1.0"), None);
}
#[test] fn converts_nvidia_versions() {
    assert_eq!(nvidia_display_version("32.0.15.6094").as_deref(), Some("560.94"));
    assert_eq!(nvidia_display_version("31.0.15.5222").as_deref(), Some("552.22"));
    assert_eq!(nvidia_display_version("1.2"), None);
}
#[test] fn detects_vendors() {
    assert_eq!(driver_vendor("NVIDIA"), Some(DriverVendor::Nvidia));
    assert_eq!(driver_vendor("Advanced Micro Devices, Inc."), Some(DriverVendor::Amd));
    assert_eq!(driver_vendor("Intel Corporation"), Some(DriverVendor::Intel));
    assert_eq!(driver_vendor("Realtek"), None);
}
#[test] fn windows_update_match_wins() {
    let wu = [AvailableDriverUpdate { hardware_id: "pci\\ven_8086&dev_46a8".into(), ..Default::default() }];
    let rows = evaluate_drivers(&fixtures::struggling_laptop(), &DriverCatalog::empty(), &wu);
    assert_eq!(status_of(&rows, "Intel(R) Iris(R) Xe Graphics"), &DriverStatus::UpdateAvailable {
        source: UpdateSource::WindowsUpdate, latest: None, url: "ms-settings:windowsupdate-optionalupdates".into() });
}
#[test] fn catalog_compares_nvidia_format() {
    let s = fixtures::healthy_desktop();
    let rows = evaluate_drivers(&s, &catalog("nvidia", "Display", "571.96"), &[]);
    assert!(matches!(status_of(&rows, "NVIDIA GeForce RTX 4070 SUPER"),
        DriverStatus::UpdateAvailable { source: UpdateSource::Catalog, latest: Some(v), .. } if v == "571.96"));
    let rows = evaluate_drivers(&s, &catalog("nvidia", "Display", "566.14"), &[]);
    assert_eq!(status_of(&rows, "NVIDIA GeForce RTX 4070 SUPER"), &DriverStatus::NoUpdateFound);
}
#[test] fn old_driver_without_catalog_is_possibly_outdated() {
    let rows = evaluate_drivers(&fixtures::struggling_laptop(), &DriverCatalog::empty(), &[]);
    assert_eq!(status_of(&rows, "Intel(R) Iris(R) Xe Graphics"), &DriverStatus::PossiblyOutdated { age_days: 883 });
}
#[test] fn unparseable_version_skips_catalog() {
    let mut s = fixtures::healthy_desktop();
    s.drivers[0].version = "n/a".into();
    let rows = evaluate_drivers(&s, &catalog("nvidia", "Display", "571.96"), &[]);
    assert_eq!(status_of(&rows, "NVIDIA GeForce RTX 4070 SUPER"), &DriverStatus::NoUpdateFound);
}
#[test] fn rows_filtered_and_sorted() {
    let mut s = fixtures::struggling_laptop();
    let sys = |m: &str, n: &str| DriverInfo { device_name: n.into(), class: "System".into(), manufacturer: m.into(), version: "1.0".into(), ..Default::default() };
    s.drivers.push(sys("Realtek", "Realtek Card Reader"));
    s.drivers.push(sys("Intel Corporation", "Intel Chipset SMBus"));
    let names: Vec<_> = evaluate_drivers(&s, &DriverCatalog::empty(), &[]).into_iter().map(|r| r.driver.device_name).collect();
    assert_eq!(names, ["Intel(R) Iris(R) Xe Graphics", "Intel(R) Wi-Fi 6 AX201 160MHz", "Intel Chipset SMBus"]);
}
#[test] fn bundled_catalog_parses() { assert_eq!(DriverCatalog::bundled().schema, 1); }
```

(The Wi-Fi driver is 1341 days old and the Iris driver 883, both `PossiblyOutdated`; the SMBus driver has no date, so `NoUpdateFound` sorts last; ties sort by device name.)

- [ ] **Step 2: Run tests to verify they fail** — `cargo test -p si-core --test drivers`.

- [ ] **Step 3: Implement `error.rs`, `drivers.rs`; write `data/driver_catalog.json`**

Rules from spec §7. `compare_versions` splits on `.`, parses each part as `u64` (any failure ⇒ `None`), pads the shorter with zeros. For NVIDIA display drivers, convert the installed version with `nvidia_display_version` before comparing to the catalog. `bundled()` uses `include_str!("../../../data/driver_catalog.json")` and falls back to `empty()` if parsing fails. The data file has schema 1 and `nvidia`/`amd`/`intel` `Display` entries with current versions from the vendors' sites and URLs `https://www.nvidia.com/en-us/drivers/`, `https://www.amd.com/en/support/download/drivers.html`, `https://www.intel.com/content/www/us/en/download-center/home.html`.

- [ ] **Step 4: Run tests to verify they pass** — 9 pass.

- [ ] **Step 5: Commit** — `git add crates/si-core data && git commit -m "feat(core): driver version comparison and update status"`

### Task 5: Hardware tiers

**Files:**
- Create: `crates/si-core/src/tier.rs`, `data/gpu_tiers.toml`, `data/cpu_tiers.toml`
- Test: `crates/si-core/tests/tier.rs`

**Interfaces:**
- Consumes: `GpuInfo`, `CpuInfo`, `SystemSnapshot::primary_gpu`, `CoreError`.
- Produces:
  - `tier::Tier { Low, Mid, High, Ultra }` (derive `Ord`, `Copy`, serde lowercase) with `fn up(self) -> Self`, `fn down(self) -> Self` (saturating), `fn label(self) -> &'static str` (`"Low"`, `"Mid"`, `"High"`, `"Ultra"`)
  - `tier::TierTable { low: Vec<String>, mid: Vec<String>, high: Vec<String>, ultra: Vec<String> }` with `fn parse(toml: &str) -> Result<Self, CoreError>`, `fn bundled_gpu() -> Self`, `fn bundled_cpu() -> Self`, `fn best_match(&self, name: &str) -> Option<Tier>`
  - `tier::gpu_tier(gpu: &GpuInfo, table: &TierTable) -> Tier`
  - `tier::cpu_tier(cpu: &CpuInfo, table: &TierTable) -> Tier`
  - `tier::overall_tier(gpu: Tier, cpu: Tier, ram_total_mb: u64) -> Tier`
  - `tier::HardwareProfile { gpu: Tier, cpu: Tier, overall: Tier, vram_mb: u64 }`; `tier::hardware_profile(s: &SystemSnapshot) -> HardwareProfile`

- [ ] **Step 1: Write the failing tests**

```rust
use si_core::{fixtures, model::*, tier::*};

fn gpu(name: &str, vram: u64, integrated: bool) -> Tier {
    gpu_tier(&GpuInfo { name: name.into(), vram_mb: vram, is_integrated: integrated, ..Default::default() }, &TierTable::bundled_gpu())
}
fn cpu(name: &str, cores: u32) -> Tier {
    cpu_tier(&CpuInfo { name: name.into(), physical_cores: cores, ..Default::default() }, &TierTable::bundled_cpu())
}

#[test] fn highest_matching_tier_wins() {
    assert_eq!(gpu("NVIDIA GeForce RTX 4070 Ti", 12282, false), Tier::Ultra);
    assert_eq!(gpu("NVIDIA GeForce RTX 4070", 12282, false), Tier::High);
    assert_eq!(gpu("AMD Radeon RX 5700 XT", 8176, false), Tier::Mid);
    assert_eq!(gpu("Radeon RX 570 Series", 4096, false), Tier::Low);
    assert_eq!(gpu("Intel(R) Arc(TM) A770 Graphics", 16256, false), Tier::Mid); // (R)/(TM) stripped before matching
}
#[test] fn mobile_gpus_step_down() {
    assert_eq!(gpu("NVIDIA GeForce RTX 4070 Laptop GPU", 8188, false), Tier::Mid);
    assert_eq!(gpu("AMD Radeon RX 6800M", 12272, false), Tier::Mid);
}
#[test] fn unknown_gpu_falls_back() {
    assert_eq!(gpu("Mystery GPU", 12288, false), Tier::High);
    assert_eq!(gpu("Mystery GPU", 8192, false), Tier::Mid);
    assert_eq!(gpu("Mystery GPU", 4096, false), Tier::Low);
    assert_eq!(gpu("Mystery iGPU", 16384, true), Tier::Low);
}
#[test] fn cpu_tiers() {
    assert_eq!(cpu("AMD Ryzen 7 7800X3D 8-Core Processor", 8), Tier::Ultra);
    assert_eq!(cpu("12th Gen Intel(R) Core(TM) i5-1235U", 10), Tier::Low);
    assert_eq!(cpu("Mystery CPU", 8), Tier::Mid);
    assert_eq!(cpu("Mystery CPU", 4), Tier::Low);
}
#[test] fn overall_rules() {
    assert_eq!(overall_tier(Tier::Ultra, Tier::Mid, 32_000), Tier::High);
    assert_eq!(overall_tier(Tier::Ultra, Tier::High, 32_000), Tier::Ultra);
    assert_eq!(overall_tier(Tier::High, Tier::High, 8_000), Tier::Mid);
    assert_eq!(overall_tier(Tier::High, Tier::High, 6_000), Tier::Low);
}
#[test] fn hybrid_laptop_uses_discrete() {
    let mut s = fixtures::struggling_laptop();
    s.gpus.push(GpuInfo { name: "NVIDIA GeForce RTX 4060 Laptop GPU".into(), vendor: Vendor::Nvidia, vram_mb: 8188, ..Default::default() });
    let hw = hardware_profile(&s);
    assert_eq!((hw.gpu, hw.vram_mb), (Tier::Low, 8188)); // RTX 4060 = Mid, Laptop ⇒ Low
}
#[test] fn fixture_profiles() {
    let hw = hardware_profile(&fixtures::healthy_desktop());
    assert_eq!((hw.gpu, hw.cpu, hw.overall), (Tier::High, Tier::Ultra, Tier::High));
    assert_eq!(hardware_profile(&SystemSnapshot::default()).overall, Tier::Low);
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p si-core --test tier`.

- [ ] **Step 3: Implement `tier.rs` and write both TOML tables** (`low = [...]`, `mid = [...]`, `high = [...]`, `ultra = [...]`) with exactly the spec §8 lists. Before matching, names are lower-cased, `(r)`, `(tm)` and `(c)` are removed and runs of spaces collapsed; then a pattern matches if it is a substring. "Mobile token" = a whitespace-separated token of 3–4 ASCII digits followed by `M` or `S`; "U-series token" = a token whose last char is `U` preceded by a digit (after stripping a leading `i3-`/`i5-`/`i7-`/`i9-`).

- [ ] **Step 4: Verify pass** — 7 pass.

- [ ] **Step 5: Commit** — `git add crates/si-core data && git commit -m "feat(core): GPU/CPU hardware tiers"`

### Task 6: Game library parsing

**Files:**
- Create: `crates/si-core/src/games/mod.rs`, `crates/si-core/src/games/library.rs`
- Test: `crates/si-core/tests/library.rs`

**Interfaces:**
- Consumes: `InstalledGame`, `Store`.
- Produces:
  - `games::library::parse_steam_library_folders(vdf: &str) -> Vec<String>`
  - `games::library::parse_steam_app_manifest(acf: &str, library_path: &str) -> Option<InstalledGame>`
  - `games::library::parse_epic_manifest(json: &str) -> Option<InstalledGame>`

- [ ] **Step 1: Write the failing tests**

```rust
use si_core::{games::library::*, model::Store};

const FOLDERS: &str = r#""libraryfolders"
{
	"0" { "path" "C:\\Program Files (x86)\\Steam" "label" "" }
	"1" { "path" "D:\\SteamLibrary" "label" "" }
}"#;
fn acf(appid: &str, flags: &str) -> String {
    format!("\"AppState\"\n{{\n\"appid\" \"{appid}\"\n\"name\" \"Counter-Strike 2\"\n\"StateFlags\" \"{flags}\"\n\"installdir\" \"Counter-Strike Global Offensive\"\n\"SizeOnDisk\" \"37000000000\"\n}}")
}

#[test] fn library_folders_lists_all_paths() {
    assert_eq!(parse_steam_library_folders(FOLDERS), ["C:\\Program Files (x86)\\Steam", "D:\\SteamLibrary"]);
}
#[test] fn app_manifest_builds_install_dir() {
    let g = parse_steam_app_manifest(&acf("730", "4"), "D:\\SteamLibrary\\").unwrap();
    assert_eq!((g.store, g.id.as_str(), g.name.as_str()), (Store::Steam, "730", "Counter-Strike 2"));
    assert_eq!(g.install_dir, "D:\\SteamLibrary\\steamapps\\common\\Counter-Strike Global Offensive");
    assert_eq!(g.size_bytes, Some(37_000_000_000));
}
#[test] fn skips_redistributables_and_incomplete() {
    assert!(parse_steam_app_manifest(&acf("228980", "4"), "C:\\Steam").is_none());
    assert!(parse_steam_app_manifest(&acf("730", "1026"), "C:\\Steam").is_none());
}
#[test] fn epic_manifest_parses() {
    let j = r#"{"DisplayName":"Fortnite","InstallLocation":"C:\\Program Files\\Epic Games\\Fortnite","AppName":"Fortnite","InstallSize":40000000000,"bIsIncompleteInstall":false}"#;
    let g = parse_epic_manifest(j).unwrap();
    assert_eq!((g.store, g.id.as_str(), g.install_dir.as_str()), (Store::Epic, "Fortnite", "C:\\Program Files\\Epic Games\\Fortnite"));
}
#[test] fn malformed_inputs_are_skipped() {
    assert!(parse_steam_library_folders("garbage {").is_empty());
    assert!(parse_steam_app_manifest("\"AppState\" { \"appid\" \"730\" }", "C:\\Steam").is_none());
    assert!(parse_epic_manifest("{not json").is_none());
    assert!(parse_epic_manifest(r#"{"DisplayName":"X","InstallLocation":"C:\\X","AppName":"X","bIsIncompleteInstall":true}"#).is_none());
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p si-core --test library`.

- [ ] **Step 3: Implement** with `keyvalues_parser::Vdf::parse` (escaped strings). Install dir = `library` with trailing `\` trimmed + `\steamapps\common\` + `installdir`. `StateFlags` present and `flags & 4 == 0` ⇒ `None`. Epic JSON via `serde_json::Value`.

- [ ] **Step 4: Verify pass** — 5 pass.

- [ ] **Step 5: Commit** — `git add crates/si-core && git commit -m "feat(core): Steam and Epic library parsing"`

### Task 7: Game profiles and setting recommendations

**Files:**
- Create: `crates/si-core/src/games/profile.rs`, `crates/si-core/src/games/recommend.rs`
- Create: `data/games/{cs2,dota2,apex,pubg,cyberpunk2077,eldenring,bg3,fortnite}.toml`
- Test: `crates/si-core/tests/recommend.rs`

**Interfaces:**
- Consumes: `Tier`, `HardwareProfile`, `InstalledGame`, `Store`, `CoreError`.
- Produces:
  - `games::profile::Impact { Low, Medium, High }` (serde lowercase)
  - `games::profile::SettingSpec { label: String, impact: Impact, vram_sensitive: bool /* default false */, low: String, mid: String, high: String, ultra: String, competitive: Option<String> }` with `fn value_for(&self, t: Tier) -> &str`
  - `games::profile::GameProfile { id: String, name: String, steam_appid: Option<u32>, epic_app_name: Option<String>, settings: Vec<SettingSpec> }` with `fn parse(toml: &str) -> Result<Self, CoreError>` (`epic_app_name = ""` becomes `None`)
  - `games::profile::ProfileLibrary { profiles: Vec<GameProfile> }` with `fn bundled() -> Self`, `fn for_game(&self, g: &InstalledGame) -> Option<&GameProfile>`
  - `games::recommend::Goal { Competitive, #[default] Balanced, Quality }` (serde lowercase)
  - `games::recommend::Reason { Tier(Tier), VramCap(u64), Competitive, QualityBoost }`
  - `games::recommend::Recommendation { label: String, value: String, impact: Impact, reason: Reason }`
  - `games::recommend::recommend(p: &GameProfile, hw: &HardwareProfile, goal: Goal) -> Vec<Recommendation>` (same order as `p.settings`)

- [ ] **Step 1: Write the failing tests**

```rust
use si_core::{games::{profile::*, recommend::*}, model::*, tier::*};

const P: &str = r#"
id = "test"
name = "Test Game"
steam_appid = 1
epic_app_name = ""
[[settings]]
label = "Shadows"
impact = "high"
low = "L"
mid = "M"
high = "H"
ultra = "U"
competitive = "Off"
[[settings]]
label = "Textures"
impact = "low"
vram_sensitive = true
low = "L"
mid = "M"
high = "H"
ultra = "U"
"#;
fn hw(overall: Tier, vram: u64) -> HardwareProfile { HardwareProfile { gpu: overall, cpu: overall, overall, vram_mb: vram } }
fn rec(goal: Goal, overall: Tier, vram: u64) -> Vec<(String, Reason)> {
    recommend(&GameProfile::parse(P).unwrap(), &hw(overall, vram), goal).into_iter().map(|r| (r.value, r.reason)).collect()
}

#[test] fn balanced_uses_overall_tier() {
    assert_eq!(rec(Goal::Balanced, Tier::High, 12_000), [("H".to_string(), Reason::Tier(Tier::High)), ("H".to_string(), Reason::Tier(Tier::High))]);
}
#[test] fn competitive_override_then_one_tier_down() {
    assert_eq!(rec(Goal::Competitive, Tier::High, 12_000), [("Off".to_string(), Reason::Competitive), ("M".to_string(), Reason::Competitive)]);
}
#[test] fn quality_boosts_only_low_impact() {
    assert_eq!(rec(Goal::Quality, Tier::High, 12_000), [("H".to_string(), Reason::Tier(Tier::High)), ("U".to_string(), Reason::QualityBoost)]);
}
#[test] fn vram_cap_applies_and_wins_over_boost() {
    assert_eq!(rec(Goal::Balanced, Tier::Ultra, 6_000)[1], ("M".to_string(), Reason::VramCap(6_000)));
    assert_eq!(rec(Goal::Quality, Tier::High, 5_000)[1], ("M".to_string(), Reason::VramCap(5_000)));
}
#[test] fn tiers_saturate() {
    assert_eq!(rec(Goal::Quality, Tier::Ultra, 16_000)[1].0, "U");
    assert_eq!(rec(Goal::Competitive, Tier::Low, 16_000)[1].0, "L");
}
#[test] fn profile_parsing_and_lookup() {
    let p = GameProfile::parse(P).unwrap();
    assert_eq!((p.steam_appid, p.epic_app_name.clone()), (Some(1), None));
    let lib = ProfileLibrary::bundled();
    let cs2 = InstalledGame { store: Store::Steam, id: "730".into(), ..Default::default() };
    let fn_ = InstalledGame { store: Store::Epic, id: "Fortnite".into(), ..Default::default() };
    assert_eq!(lib.for_game(&cs2).unwrap().id, "cs2");
    assert_eq!(lib.for_game(&fn_).unwrap().id, "fortnite");
}
#[test] fn bundled_profiles_are_complete() {
    let lib = ProfileLibrary::bundled();
    assert_eq!(lib.profiles.len(), 8);
    let mut ids: Vec<_> = lib.profiles.iter().map(|p| p.id.clone()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 8);
    for p in &lib.profiles {
        assert!(p.steam_appid.is_some() || p.epic_app_name.is_some(), "{}", p.id);
        assert!(p.settings.len() >= 6, "{}", p.id);
        assert!(p.settings.iter().all(|s| [&s.low, &s.mid, &s.high, &s.ultra].iter().all(|v| !v.is_empty())), "{}", p.id);
    }
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p si-core --test recommend`.

- [ ] **Step 3: Implement** per spec §8 "Recommendation per setting". Under Competitive with no override, the reason is still `Competitive`. Reason precedence: Competitive > VramCap (only when the cap lowered the tier) > QualityBoost > Tier. `bundled()` holds a `const` array of the 8 `include_str!`s; a profile that fails to parse is skipped (and caught by the completeness test).

- [ ] **Step 4: Write the 8 profiles** (ids and store ids in spec §8). For each game list at least 6 of its most FPS-relevant settings, labels exactly as in the game's current graphics menu, `impact` from published performance guides, `competitive` only where pros commonly use a fixed value (e.g. CS2 "Boost Player Contrast" = "Enabled", "Wait for Vertical Sync" = "Disabled"). Mark texture/model quality settings `vram_sensitive = true`.

- [ ] **Step 5: Verify pass** — 7 pass. Then `cargo test -p si-core` → every core test passes.

- [ ] **Step 6: Commit** — `git add crates/si-core data && git commit -m "feat(core): game profiles and settings recommendations"`

---

## Phase 2 — Data collection (`si-probe`)

### Task 8: Probe trait, sysinfo probe and fixture probe

**Files:**
- Modify: `Cargo.toml` (add `crates/si-probe` member)
- Create: `crates/si-probe/Cargo.toml` (deps: `si-core`, `sysinfo`, `serde`, `serde_json`, `thiserror`, `tracing`), `crates/si-probe/src/{lib,generic,fixture}.rs`
- Test: `crates/si-probe/tests/generic.rs`

**Interfaces:**
- Consumes: `SystemSnapshot`, `InstalledGame`, `LiveSample`, `ProcessUsage`, `fixtures`.
- Produces:
  - `ScanStep { Hardware, Memory, Storage, Displays, OsSettings, Processes, Drivers, Games }` (derive `Copy, Debug, PartialEq, Eq, Serialize, Deserialize`) with `fn label(self) -> &'static str` (`"Scanning hardware…"`, `"Scanning memory…"`, `"Scanning storage…"`, `"Scanning displays…"`, `"Reading Windows settings…"`, `"Checking background apps…"`, `"Reading drivers…"`, `"Finding games…"`)
  - `ScanWarning { step: ScanStep, message: String }`; `ScanResult { snapshot: SystemSnapshot, games: Vec<InstalledGame>, warnings: Vec<ScanWarning> }` (serde, `Clone`, `Default`)
  - `ProbeError { Io(#[from] std::io::Error), Json(#[from] serde_json::Error), Windows(String), Network(String) }`
  - `trait SystemProbe { fn scan(&mut self, progress: &mut dyn FnMut(ScanStep)) -> ScanResult; fn sample(&mut self) -> LiveSample; }`
  - `pub type ProbeFactory = Box<dyn FnOnce() -> Box<dyn SystemProbe> + Send>`
  - `default_probe() -> Box<dyn SystemProbe>` (Windows: `WindowsProbe`, Task 10; elsewhere: `GenericProbe`)
  - `generic::GenericProbe::new() -> Self`; `generic::normalize_cpu(raw_pct: f32, logical_cores: u32) -> f32`; `generic::select_processes(all: Vec<ProcessUsage>) -> Vec<ProcessUsage>`
  - `fixture::FixtureProbe::load(path: &Path) -> Result<Self, ProbeError>`; `FixtureProbe::new(result: ScanResult) -> Self`

- [ ] **Step 1: Write the failing tests**

```rust
use si_probe::{fixture::FixtureProbe, generic::*, ScanResult, ScanStep, SystemProbe};
use si_core::{fixtures, model::ProcessUsage};

#[test] fn normalize_cpu_divides_by_cores() {
    assert_eq!(normalize_cpu(400.0, 8), 50.0);
    assert_eq!(normalize_cpu(900.0, 8), 100.0);
    assert_eq!(normalize_cpu(50.0, 0), 0.0);
}
#[test] fn select_processes_unions_cpu_and_memory_tops() {
    let p = |pid, cpu, mem| ProcessUsage { pid, name: format!("p{pid}"), cpu_pct: cpu, mem_mb: mem, gpu_pct: None };
    let mut all: Vec<_> = (0..40).map(|i| p(i, i as f32, 10 + i as u64)).collect();
    all.push(p(100, 0.0, 9000));
    let sel = select_processes(all);
    assert_eq!(sel.len(), 26);
    assert!(sel.iter().any(|x| x.pid == 100));
    assert_eq!(sel[0].pid, 39);
}
#[test] fn generic_probe_reads_this_machine() {
    let mut steps = vec![];
    let r = GenericProbe::new().scan(&mut |s| steps.push(s));
    assert!(r.snapshot.cpu.logical_cores > 0 && r.snapshot.memory.total_mb > 0);
    assert!(!r.snapshot.processes.is_empty());
    assert_eq!(steps.first(), Some(&ScanStep::Hardware));
}
#[test] fn generic_sample_has_cpu_and_ram() {
    let mut p = GenericProbe::new();
    p.sample();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let s = p.sample();
    assert!(s.ram_total_mb > 0 && (0.0..=100.0).contains(&s.cpu_total_pct));
}
#[test] fn fixture_probe_loads_snapshot_or_scan_result() {
    let dir = std::env::temp_dir().join(format!("si-probe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let snap = dir.join("snap.json");
    std::fs::write(&snap, serde_json::to_string(&fixtures::healthy_desktop()).unwrap()).unwrap();
    let full = dir.join("full.json");
    std::fs::write(&full, serde_json::to_string(&ScanResult { snapshot: fixtures::struggling_laptop(), ..Default::default() }).unwrap()).unwrap();
    assert_eq!(FixtureProbe::load(&snap).unwrap().scan(&mut |_| {}).snapshot, fixtures::healthy_desktop());
    assert_eq!(FixtureProbe::load(&full).unwrap().scan(&mut |_| {}).snapshot.machine.hostname, "LAPTOP-STUDENT");
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p si-probe --test generic`.

- [ ] **Step 3: Implement**

`GenericProbe` holds `sysinfo::System` + `sysinfo::Disks`; `scan` refreshes CPU and processes twice, `sysinfo::MINIMUM_CPU_UPDATE_INTERVAL` apart, and fills hostname, OS name, CPU name/cores, memory total/used, disks (media `Unknown`), processes; it calls `progress` for every `ScanStep` in declaration order. `select_processes` = top 25 by CPU ∪ top 25 by memory, dedup by pid, sorted by CPU descending. `FixtureProbe::load` tries `ScanResult` JSON first, then `SystemSnapshot`; its `sample()` returns a deterministic wave (`gpu_pct = 70 + 25·sin(t)`, etc.) so the UI is alive in dev mode. `FixtureProbe` returns the file's snapshot unchanged (its `taken_at_unix` keeps driver ages reproducible); `GenericProbe` sets `taken_at_unix` to the current time. `GenericProbe::sample` fills `t_ms`, `cpu_total_pct`, `cpu_max_core_pct` (busiest logical core), RAM and the top 8 processes by CPU; Windows reuses it in Task 11.

- [ ] **Step 4: Verify pass** — 5 pass.

- [ ] **Step 5: Commit** — `git add Cargo.toml crates/si-probe && git commit -m "feat(probe): probe trait, sysinfo and fixture probes"`

### Task 9: WMI and DXGI collectors

**Files:**
- Create: `crates/si-probe/src/mapping.rs` (all OS), `crates/si-probe/src/win/{mod,wmi,dxgi}.rs` (cfg windows)
- Modify: `crates/si-probe/Cargo.toml` (`[target.'cfg(windows)'.dependencies]` `wmi`, `winreg`, `windows`, `nvml-wrapper`)
- Test: `crates/si-probe/tests/mapping.rs`, `crates/si-probe/tests/windows_scan.rs` (`#![cfg(windows)]`)

**Interfaces:**
- Consumes: Task 1 model, Task 8 `ScanWarning`, `ScanStep`.
- Produces (`mapping`, all OS; row structs derive `Deserialize, Default` with `#[serde(rename = "<Class>", rename_all = "PascalCase")]` and `Option` fields):
  - `Win32Processor { name, number_of_cores, number_of_logical_processors, max_clock_speed }`
  - `Win32PhysicalMemory { capacity: Option<String> /* uint64 arrives as string */, speed: Option<u32>, configured_clock_speed: Option<u32>, #[serde(rename = "SMBIOSMemoryType")] smbios_memory_type: Option<u32> }`
  - `Win32VideoController { name, driver_version, driver_date }`
  - `Win32PnPSignedDriver { device_name, device_class, manufacturer, driver_version, driver_date, #[serde(rename = "HardWareID")] hardware_id }`
  - `MsftPhysicalDisk { device_id: Option<String>, media_type: Option<u16>, bus_type: Option<u16> }`; `MsftPartition { disk_number: Option<u32>, drive_letter: Option<u16> }`
  - `Win32DeviceGuard { security_services_running: Option<Vec<u32>> }`
  - `parse_wmi_date(s: &str) -> Option<String>`; `ole_date_to_ymd(days: f64) -> Option<String>`
  - `memory_module(row: &Win32PhysicalMemory) -> MemoryModule`
  - `vendor_from_pci_id(id: u32) -> Vendor`; `is_skipped_adapter(vendor_id: u32, flags: u32) -> bool`; `integrated_by_name(name: &str) -> bool`
  - `disk_media(media_type: u16, bus_type: u16) -> DiskMedia`; `media_by_letter(disks: &[MsftPhysicalDisk], parts: &[MsftPartition]) -> HashMap<char, DiskMedia>`
  - `driver_info(row: &Win32PnPSignedDriver) -> Option<DriverInfo>`
  - `attach_gpu_drivers(gpus: &mut [GpuInfo], controllers: &[Win32VideoController])`
  - `memory_integrity_on(row: &Win32DeviceGuard) -> Option<bool>`
- Produces (`si_probe::win`, cfg windows): `wmi::query_in<T: DeserializeOwned>(namespace: &str, warnings: &mut Vec<ScanWarning>, step: ScanStep) -> Vec<T>` (the single entry point for every WMI query: connection or query failure ⇒ one `ScanWarning`, empty vec), `wmi::collect_wmi(snap: &mut SystemSnapshot, warnings: &mut Vec<ScanWarning>)`, `dxgi::enumerate_gpus() -> windows::core::Result<Vec<GpuInfo>>`

- [ ] **Step 1: Write the failing mapping tests**

```rust
use si_probe::mapping::*;
use si_core::model::*;

#[test] fn parses_wmi_and_ole_dates() {
    assert_eq!(parse_wmi_date("20240115000000.000000-000").as_deref(), Some("2024-01-15"));
    assert_eq!(parse_wmi_date("garbage"), None);
    assert_eq!(ole_date_to_ymd(45658.0).as_deref(), Some("2025-01-01"));
}
#[test] fn ddr5_module_maps() {
    let m = memory_module(&Win32PhysicalMemory { capacity: Some("17179869184".into()), speed: Some(6000), configured_clock_speed: Some(4800), smbios_memory_type: Some(34) });
    assert_eq!(m, MemoryModule { capacity_mb: 16384, rated_mhz: 6000, configured_mhz: 4800, generation: MemoryGen::Ddr5 });
}
#[test] fn skips_microsoft_and_software_adapters() {
    assert!(is_skipped_adapter(0x1414, 0));
    assert!(is_skipped_adapter(0x10DE, 2));
    assert!(!is_skipped_adapter(0x10DE, 0));
    assert_eq!(vendor_from_pci_id(0x1002), Vendor::Amd);
}
#[test] fn integrated_names() {
    for n in ["Intel(R) UHD Graphics 770", "Intel(R) Iris(R) Xe Graphics", "AMD Radeon(TM) Graphics", "AMD Radeon 780M Graphics"] { assert!(integrated_by_name(n), "{n}"); }
    assert!(!integrated_by_name("AMD Radeon RX 7800 XT"));
}
#[test] fn disk_media_by_letter() {
    assert_eq!(disk_media(4, 17), DiskMedia::Nvme);
    assert_eq!(disk_media(3, 11), DiskMedia::Hdd);
    let disks = [MsftPhysicalDisk { device_id: Some("1".into()), media_type: Some(3), bus_type: Some(11) }];
    let parts = [MsftPartition { disk_number: Some(1), drive_letter: Some('D' as u16) }, MsftPartition { disk_number: Some(1), drive_letter: Some(0) }];
    assert_eq!(media_by_letter(&disks, &parts).get(&'D'), Some(&DiskMedia::Hdd));
}
#[test] fn drivers_map_and_skip_incomplete() {
    let row = Win32PnPSignedDriver { device_name: Some("NVIDIA GeForce RTX 4070".into()), device_class: Some("DISPLAY".into()), manufacturer: Some("NVIDIA".into()), driver_version: Some("32.0.15.6094".into()), driver_date: Some("20240910000000.******+***".into()), hardware_id: Some("PCI\\VEN_10DE&DEV_2786".into()) };
    let d = driver_info(&row).unwrap();
    assert_eq!((d.class.as_str(), d.date.as_deref()), ("Display", Some("2024-09-10")));
    assert!(driver_info(&Win32PnPSignedDriver { driver_version: None, ..row }).is_none());
}
#[test] fn attaches_gpu_driver_by_name() {
    let mut g = [GpuInfo { name: "NVIDIA GeForce RTX 4070".into(), ..Default::default() }];
    attach_gpu_drivers(&mut g, &[Win32VideoController { name: Some("nvidia geforce rtx 4070 ".into()), driver_version: Some("32.0.15.6094".into()), driver_date: Some("20240910000000.000000-000".into()) }]);
    assert_eq!(g[0].driver_date.as_deref(), Some("2024-09-10"));
}
#[test] fn memory_integrity_flag() {
    assert_eq!(memory_integrity_on(&Win32DeviceGuard { security_services_running: Some(vec![1, 2]) }), Some(true));
    assert_eq!(memory_integrity_on(&Win32DeviceGuard { security_services_running: None }), None);
}
```

Device class mapping: WMI returns upper-case classes (`DISPLAY`, `NET`, `MEDIA`, `SCSIADAPTER`, `HDC`, `BLUETOOTH`, `SYSTEM`); `driver_info` normalizes to the spec spelling (`Display`, `Net`, `Media`, `SCSIAdapter`, `HDC`, `Bluetooth`, `System`), other classes title-cased.

- [ ] **Step 2: Run to verify failure** — `cargo test -p si-probe --test mapping`.

- [ ] **Step 3: Implement `mapping.rs`** (pure functions; the `******+***` WMI date variant must parse).

- [ ] **Step 4: Verify pass** — 8 pass.

- [ ] **Step 5: Implement `win/wmi.rs` and `win/dxgi.rs`**

`collect_wmi` queries `root\cimv2`, `root\Microsoft\Windows\Storage` and `root\Microsoft\Windows\DeviceGuard`, always through `query_in` (one thread-local `wmi::COMLibrary`, created on first use), so a failing namespace or class only adds a warning. `enumerate_gpus` uses `CreateDXGIFactory1` → `EnumAdapters1` → `GetDesc1`, skips via `is_skipped_adapter`, sets `vram_mb = DedicatedVideoMemory / 1 MiB`, and decides `is_integrated` with `D3D12CreateDevice(adapter, D3D_FEATURE_LEVEL_11_0)` + `CheckFeatureSupport(D3D12_FEATURE_ARCHITECTURE)` → `UMA`, falling back to `integrated_by_name`.

- [ ] **Step 6: Windows integration test** (`tests/windows_scan.rs`)

```rust
#![cfg(windows)]
#[test] fn wmi_fills_cpu_and_drivers() {
    let mut s = si_core::model::SystemSnapshot::default();
    let mut w = vec![];
    si_probe::win::wmi::collect_wmi(&mut s, &mut w);
    assert!(!s.cpu.name.is_empty() && s.cpu.logical_cores > 0, "{w:?}");
    assert!(!s.drivers.is_empty());
}
#[test] fn scan_survives_failing_collectors() {
    let mut w = vec![];
    let rows: Vec<si_probe::mapping::Win32Processor> = si_probe::win::wmi::query_in(r"root\DoesNotExist", &mut w, si_probe::ScanStep::Hardware);
    assert!(rows.is_empty() && w.len() == 1);
}
#[test] #[ignore = "needs a real GPU"] fn real_gpu_detected() {
    assert!(!si_probe::win::dxgi::enumerate_gpus().unwrap().is_empty());
}
```

Run on Windows: `cargo test -p si-probe --test windows_scan` → 2 pass, 1 ignored. On a gaming PC also run `-- --ignored`.

- [ ] **Step 7: Commit** — `git add crates/si-probe && git commit -m "feat(probe): WMI and DXGI hardware collectors"`

### Task 10: Windows settings, power, displays, game discovery and `WindowsProbe`

**Files:**
- Create: `crates/si-probe/src/winsettings.rs` (all OS), `crates/si-probe/src/win/{registry,power,display,games}.rs`
- Modify: `crates/si-probe/src/win/mod.rs` (`WindowsProbe`), `crates/si-probe/src/lib.rs` (`default_probe`)
- Test: `crates/si-probe/tests/winsettings.rs`; extend `crates/si-probe/tests/windows_scan.rs`

**Interfaces:**
- Consumes: Tasks 6, 8, 9.
- Produces (`winsettings`, all OS):
  - `power_plan_from_guid(guid: &str) -> PowerPlan` (braces and case ignored; unknown ⇒ `Custom`)
  - `os_name_for_build(build: u32) -> &'static str`
  - `game_mode_from(v: Option<u32>) -> Option<bool>`; `recording_from(v: Option<u32>) -> Option<bool>`; `hags_from(v: Option<u32>) -> Option<bool>`
  - `startup_enabled(approved: Option<&[u8]>) -> bool`
  - `battery_state(battery_flag: u8, ac_line_status: u8) -> (Option<bool>, Option<bool>)` → `(is_laptop, on_battery)`
  - `max_hz_for_resolution(modes: &[(u32, u32, u32)], width: u32, height: u32) -> u32`
  - `discover_games_in(steam_root: Option<&Path>, epic_manifests: Option<&Path>) -> (Vec<InstalledGame>, Vec<ScanWarning>)` (filesystem only, so it is testable on all OS)
- Produces (cfg windows): `win::WindowsProbe::new() -> Self` implementing `SystemProbe::scan` (sample delegates to `GenericProbe` until Task 11).

- [ ] **Step 1: Write the failing tests**

```rust
use si_probe::winsettings::*;
use si_core::model::PowerPlan;

#[test] fn power_plan_by_guid() {
    assert_eq!(power_plan_from_guid("{8C5E7FDA-E8BF-4A96-9A85-A6E23A8C635C}"), PowerPlan::HighPerformance);
    assert_eq!(power_plan_from_guid("a1841308-3541-4fab-bc81-f71556f20b4a"), PowerPlan::PowerSaver);
    assert_eq!(power_plan_from_guid("00000000-0000-0000-0000-000000000000"), PowerPlan::Custom);
}
#[test] fn windows_11_by_build() { assert_eq!(os_name_for_build(22631), "Windows 11"); assert_eq!(os_name_for_build(19045), "Windows 10"); }
#[test] fn registry_defaults() {
    assert_eq!(game_mode_from(None), Some(true));
    assert_eq!(game_mode_from(Some(0)), Some(false));
    assert_eq!(recording_from(None), Some(false));
    assert_eq!((hags_from(Some(2)), hags_from(Some(1)), hags_from(None)), (Some(true), Some(false), None));
}
#[test] fn startup_approved_bytes() {
    assert!(startup_enabled(None));
    assert!(startup_enabled(Some(&[2, 0, 0])));
    assert!(!startup_enabled(Some(&[3, 0, 0])));
    assert!(!startup_enabled(Some(&[7, 0, 0])));
}
#[test] fn battery_states() {
    assert_eq!(battery_state(128, 1), (Some(false), Some(false)));
    assert_eq!(battery_state(8, 0), (Some(true), Some(true)));
    assert_eq!(battery_state(255, 255), (None, None));
}
#[test] fn max_hz_only_at_current_resolution() {
    let modes = [(1920, 1080, 60), (1920, 1080, 144), (1280, 720, 240)];
    assert_eq!(max_hz_for_resolution(&modes, 1920, 1080), 144);
}
#[test] fn missing_library_folder_is_skipped() {
    let root = std::env::temp_dir().join(format!("si-steam-{}", std::process::id()));
    let apps = root.join("steamapps");
    std::fs::create_dir_all(&apps).unwrap();
    let lib = root.to_string_lossy().replace('\\', "\\\\");
    std::fs::write(apps.join("libraryfolders.vdf"), format!("\"libraryfolders\" {{ \"0\" {{ \"path\" \"{lib}\" }} \"1\" {{ \"path\" \"Z:\\\\Gone\" }} }}")).unwrap();
    std::fs::write(apps.join("appmanifest_730.acf"), "\"AppState\" { \"appid\" \"730\" \"name\" \"Counter-Strike 2\" \"StateFlags\" \"4\" \"installdir\" \"cs2\" }").unwrap();
    let (games, _warnings) = discover_games_in(Some(&root), None);
    assert_eq!(games.len(), 1);
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p si-probe --test winsettings`.

- [ ] **Step 3: Implement `winsettings.rs`** per spec §4. `startup_enabled`: `None` ⇒ true; odd first byte ⇒ false.

- [ ] **Step 4: Verify pass** — 7 pass.

- [ ] **Step 5: Implement the Windows collectors and `WindowsProbe::scan`**

Registry reads via `winreg` (paths in spec §4); power via `GetSystemPowerStatus` and `PowerGetActiveScheme` (free the returned GUID pointer with `LocalFree`); displays via `EnumDisplayDevicesW` (attached-to-desktop only) + `EnumDisplaySettingsW`; games via `discover_games_in(SteamPath from HKCU\Software\Valve\Steam, %ProgramData%\Epic\EpicGamesLauncher\Data\Manifests)`. `scan` order and progress: Hardware (generic CPU + WMI CPU + DXGI GPUs + `attach_gpu_drivers`), Memory, Storage (sysinfo + `media_by_letter`), Displays, OsSettings, Processes, Drivers, Games. Every collector is wrapped so an error becomes a `ScanWarning`. `default_probe()` returns `WindowsProbe` on Windows.

- [ ] **Step 6: Windows integration tests** (append to `windows_scan.rs`)

```rust
#[test] fn windows_probe_full_scan() {
    use si_probe::SystemProbe;
    let r = si_probe::default_probe().scan(&mut |_| {});
    assert!(r.snapshot.machine.os_build >= 19041);
    assert_ne!(r.snapshot.os.power_plan, si_core::model::PowerPlan::Unknown);
}
```

- [ ] **Step 7: Commit** — `git add crates/si-probe && git commit -m "feat(probe): Windows settings, displays, games and full WindowsProbe scan"`

### Task 11: Live sampler (PDH, NVML)

**Files:**
- Create: `crates/si-probe/src/gpu_counters.rs` (all OS), `crates/si-probe/src/win/{pdh,nvml}.rs`
- Modify: `crates/si-probe/src/win/mod.rs` (`WindowsProbe::sample`)
- Test: `crates/si-probe/tests/gpu_counters.rs`; extend `windows_scan.rs`

**Interfaces:**
- Produces:
  - `gpu_counters::EngineInstance { pid: u32, luid: String, is_3d: bool }`; `parse_gpu_engine_instance(name: &str) -> Option<EngineInstance>`
  - `gpu_counters::GpuAggregate { total_pct: f32, per_pid: HashMap<u32, f32> }`; `aggregate_gpu(readings: &[(String, f64)]) -> GpuAggregate` (`total_pct` = max over adapters of the sum of 3D-engine values, clamped to 100)
  - (cfg windows) `pdh::PdhSampler::new() -> windows::core::Result<Self>`, `fn collect(&mut self) -> PdhReading { gpu_instances: Vec<(String, f64)>, vram_used_mb: Option<u64>, cpu_perf_pct: Option<f32> }`
  - (cfg windows) `nvml::NvmlReader::new() -> Option<Self>`, `fn read(&self) -> Option<NvidiaReading { temp_c: f32, util_pct: f32, mem_used_mb: u64, mem_total_mb: u64, throttling: bool }>`

- [ ] **Step 1: Write the failing tests**

```rust
use si_probe::gpu_counters::*;

#[test] fn parses_engine_instance() {
    let e = parse_gpu_engine_instance("pid_1234_luid_0x00000000_0x0000D1A5_phys_0_eng_0_engtype_3D").unwrap();
    assert_eq!((e.pid, e.luid.as_str(), e.is_3d), (1234, "0x00000000_0x0000D1A5", true));
    assert!(!parse_gpu_engine_instance("pid_1_luid_0x0_0x1_phys_0_eng_3_engtype_Copy").unwrap().is_3d);
    assert!(parse_gpu_engine_instance("_Total").is_none());
}
#[test] fn aggregates_3d_per_adapter_and_pid() {
    let r = [
        ("pid_10_luid_0x0_0xA_phys_0_eng_0_engtype_3D".to_string(), 40.0),
        ("pid_11_luid_0x0_0xA_phys_0_eng_0_engtype_3D".to_string(), 30.0),
        ("pid_10_luid_0x0_0xA_phys_0_eng_2_engtype_Copy".to_string(), 90.0),
        ("pid_12_luid_0x0_0xB_phys_0_eng_0_engtype_3D".to_string(), 5.0),
    ];
    let a = aggregate_gpu(&r);
    assert_eq!(a.total_pct, 70.0);
    assert_eq!(a.per_pid[&10], 40.0);
}
#[test] fn total_is_clamped() {
    let r = [("pid_1_luid_0x0_0xA_phys_0_eng_0_engtype_3D".to_string(), 80.0), ("pid_2_luid_0x0_0xA_phys_0_eng_0_engtype_3D".to_string(), 60.0)];
    assert_eq!(aggregate_gpu(&r).total_pct, 100.0);
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p si-probe --test gpu_counters`.

- [ ] **Step 3: Implement `gpu_counters.rs`** — 3 pass.

- [ ] **Step 4: Implement PDH, NVML and `sample()`**

`PdhSampler::new` opens one query and adds, with **`PdhAddEnglishCounterW`**, `\GPU Engine(*engtype_3D)\Utilization Percentage`, `\GPU Adapter Memory(*)\Dedicated Usage`, `\Processor Information(_Total)\% Processor Performance`; calls `PdhCollectQueryData` once. `collect` calls it again and reads arrays with `PdhGetFormattedCounterArrayW(PDH_FMT_DOUBLE)`. NVML `throttling` = reasons ∩ {`HW_SLOWDOWN`, `HW_THERMAL_SLOWDOWN`, `SW_THERMAL_SLOWDOWN`} non-empty. `WindowsProbe::sample` = `GenericProbe::sample` (CPU, RAM, top processes) + PDH (GPU %, VRAM, per-process `gpu_pct`, `cpu_perf_pct`) + NVML when present (NVML values override PDH for GPU %, VRAM, temp, throttling). `t_ms` = ms since probe creation.

- [ ] **Step 5: Windows integration test**

```rust
#[test] fn windows_sample_has_cpu_perf() {
    use si_probe::SystemProbe;
    let mut p = si_probe::default_probe();
    p.sample();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let s = p.sample();
    assert!(s.cpu_perf_pct.is_some(), "English PDH counter missing: {s:?}");
}
```

- [ ] **Step 6: Commit** — `git add crates/si-probe && git commit -m "feat(probe): live sampler with PDH GPU counters and NVML"`

### Task 12: Driver catalog fetch and Windows Update search

**Files:**
- Create: `crates/si-probe/src/catalog.rs`, `crates/si-probe/src/win/wua.rs`
- Modify: `crates/si-probe/src/lib.rs` (re-export `search_windows_update_drivers`)
- Test: `crates/si-probe/tests/catalog.rs`

**Interfaces:**
- Consumes: `DriverCatalog`, `AvailableDriverUpdate`, `mapping::ole_date_to_ymd`.
- Produces:
  - `catalog::CatalogSource { Remote, Bundled }` (serde, `Copy`)
  - `catalog::fetch_catalog(url: &str, timeout: Duration) -> Result<DriverCatalog, ProbeError>` (error if HTTP status is not 200 or `schema != 1`)
  - `catalog::load_catalog(url: &str) -> (DriverCatalog, CatalogSource)` (10 s timeout; any error ⇒ `DriverCatalog::bundled()`, `Bundled`)
  - `search_windows_update_drivers() -> Result<Vec<AvailableDriverUpdate>, ProbeError>` (non-Windows: `Ok(vec![])`)

- [ ] **Step 1: Write the failing tests** (a one-shot local HTTP server, no extra crates)

```rust
use si_probe::catalog::*;
use std::{io::{Read, Write}, net::TcpListener, thread};

fn serve_once(body: &'static str) -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/catalog.json", l.local_addr().unwrap());
    thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        let mut buf = [0u8; 1024];
        let _ = s.read(&mut buf);
        let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
    });
    url
}

#[test] fn remote_catalog_used_when_valid() {
    let url = serve_once(r#"{"schema":1,"updated":"2030-01-01","entries":[]}"#);
    let (c, src) = load_catalog(&url);
    assert_eq!((src, c.updated.as_str()), (CatalogSource::Remote, "2030-01-01"));
}
#[test] fn wrong_schema_falls_back() {
    let url = serve_once(r#"{"schema":2,"updated":"2030-01-01","entries":[]}"#);
    assert_eq!(load_catalog(&url).1, CatalogSource::Bundled);
}
#[test] fn offline_falls_back_to_bundled() {
    assert_eq!(load_catalog("http://127.0.0.1:9/none.json").1, CatalogSource::Bundled);
}
#[test] #[ignore = "slow; needs network and Windows Update"] fn wua_search_runs() {
    assert!(si_probe::search_windows_update_drivers().is_ok());
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p si-probe --test catalog`.

- [ ] **Step 3: Implement `catalog.rs`** with `reqwest::blocking::Client::builder().timeout(timeout)` — 3 pass. (reqwest honours `HTTP_PROXY`; if a proxy is set locally, run the tests with `NO_PROXY=127.0.0.1`.)

- [ ] **Step 4: Implement `win/wua.rs`**

On the calling thread: `CoInitializeEx(None, COINIT_MULTITHREADED)`; `CoCreateInstance::<_, IUpdateSession>(&UpdateSession, None, CLSCTX_INPROC_SERVER)`; `CreateUpdateSearcher()`; `Search(&BSTR::from("IsInstalled=0 and Type='Driver'"))`; iterate `Updates()`, `cast::<IWindowsDriverUpdate>()`, map `Title`, `DriverHardwareID`, `DriverClass`, `DriverModel`, `DriverVerDate` (via `ole_date_to_ymd`); `CoUninitialize` on exit. COM errors ⇒ `ProbeError::Windows(message)`.

- [ ] **Step 5: Commit** — `git add crates/si-probe && git commit -m "feat(probe): driver catalog fetch and Windows Update driver search"`

### Task 13: PresentMon runner

**Files:**
- Create: `crates/si-probe/src/presentmon.rs`
- Test: `crates/si-probe/tests/presentmon.rs`

**Interfaces:**
- Consumes: `frames::FrameTimeParser`, `frames::FrameWindow`.
- Produces:
  - `presentmon::presentmon_args(process_name: &str) -> Vec<String>`
  - `presentmon::PresentMonSession { fn start(exe: &Path, process_name: &str) -> Result<Self, ProbeError>; fn start_command(cmd: std::process::Command) -> Result<Self, ProbeError>; fn drain_into(&self, window: &mut FrameWindow) -> usize; fn is_running(&mut self) -> bool }` (`Drop` kills the child)

- [ ] **Step 1: Write the failing tests**

```rust
use si_probe::presentmon::*;
use si_core::frames::FrameWindow;

#[test] fn args_match_spec() {
    assert_eq!(presentmon_args("game.exe"), ["--process_name", "game.exe", "--output_stdout", "--no_console_stats",
        "--stop_existing_session", "--session_name", "SpeedImprover", "--terminate_on_proc_exit"]);
}

#[test] fn streams_frame_times_from_child() {
    #[cfg(unix)]
    let cmd = { let mut c = std::process::Command::new("sh"); c.args(["-c", "printf 'Application,MsBetweenPresents\\ngame.exe,10.0\\ngame.exe,20.0\\n'"]); c };
    #[cfg(windows)]
    let cmd = { let mut c = std::process::Command::new("cmd"); c.args(["/C", "echo Application,MsBetweenPresents& echo game.exe,10.0& echo game.exe,20.0"]); c };
    let s = PresentMonSession::start_command(cmd).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
    let mut w = FrameWindow::new(5000.0);
    assert_eq!(s.drain_into(&mut w), 2);
    assert_eq!(w.as_slice(), [10.0, 20.0]);
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p si-probe --test presentmon`.

- [ ] **Step 3: Implement** — stdout piped; a reader thread feeds lines (trimmed) to a `FrameTimeParser` and sends `f64`s over a `std::sync::mpsc` channel; on Windows set `CREATE_NO_WINDOW` (`0x0800_0000`) via `CommandExt::creation_flags` (if `cmd` quoting breaks the test command, pass the `/C` string with `CommandExt::raw_arg`). `start` = `start_command(Command::new(exe).args(presentmon_args(..)))`.

- [ ] **Step 4: Verify pass** — 2 pass. Then `cargo test -p si-probe` → all pass.

- [ ] **Step 5: Commit** — `git add crates/si-probe && git commit -m "feat(probe): PresentMon frame-time capture"`

---

## Phase 3 — App (`si-app`)

### Task 14: App shell — state, theme, settings, worker, navigation, Settings page

**Files:**
- Modify: `Cargo.toml` (add `crates/si-app`)
- Create: `crates/si-app/Cargo.toml` (lib `si_app` + bin `speedimprover`; dev-deps `egui_kittest`)
- Create: `crates/si-app/src/{main,lib,app,state,theme,settings,worker,nav,testing}.rs`, `crates/si-app/src/pages/{mod,settings}.rs`, `crates/si-app/src/widgets/{mod,toasts}.rs`, `crates/si-app/assets/demo.json`
- Create: `crates/si-app/tests/common/mod.rs` — re-exports (`egui_kittest::{Harness, kittest::Queryable}`, `si_app::{app::SpeedImproverApp, nav::Page, settings::*, state::*, testing, worker::*}`, `si_core::fixtures`) and `pub fn harness(state: AppState) -> Harness<'static, SpeedImproverApp>` (= `Harness::new_ui_state(|ui, app| app.show(ui), SpeedImproverApp::headless(state))`). The file starts with `#![allow(dead_code, unused_imports)]` (each test crate uses a different subset, and clippy runs with `-D warnings`). Every si-app test file starts with `mod common; use common::*;`.
- Test: `crates/si-app/tests/shell.rs`

**Interfaces:**
- Consumes: everything from `si-core`; `si_probe::{SystemProbe, ProbeFactory, ScanResult, ScanStep, catalog::*, presentmon::*, search_windows_update_drivers, default_probe, fixture::FixtureProbe}`.
- Produces:
  - `theme::Palette { bg, card, card_raised, border, text, text_muted, accent, accent_strong, success, warning, danger: egui::Color32 }` with `fn dark()`, `fn light()` (hex values from spec §9); `theme::ThemeChoice { #[default] Dark, Light }`; `theme::apply(ctx: &egui::Context, p: &Palette)`; `theme::install_fonts(ctx: &egui::Context)` (egui-phosphor Regular)
  - `settings::DEFAULT_CATALOG_URL: &str` = `"https://raw.githubusercontent.com/supernover80/speedimprover/main/data/driver_catalog.json"`
  - `settings::Settings { theme: ThemeChoice, favorites: Vec<String>, goal: Goal, presentmon_path: Option<PathBuf>, catalog_url: String }` (`#[serde(default)]`, `Default` uses `DEFAULT_CATALOG_URL`) with `fn load(path: &Path) -> Settings` and `fn save(&self, path: &Path) -> anyhow::Result<()>`
  - `settings::AppPaths { config_file: PathBuf, last_scan: PathBuf, log_file: PathBuf }` with `fn from_project_dirs() -> Option<Self>`, `fn in_dir(dir: &Path) -> Self`
  - `worker::WorkerCommand` (derive `Debug, Clone, PartialEq`) `{ Scan, StartLive, StopLive, CheckDrivers { catalog_url: String }, StartFps { presentmon: PathBuf, process_name: String }, StopFps, Shutdown }`
  - `worker::WorkerEvent { ScanProgress(ScanStep), ScanDone(Box<ScanResult>), Sample(LiveSample), Frames { stats: FrameStats, frame_times: Vec<f64> }, Drivers { rows: Vec<DriverRow>, catalog: CatalogSource, checked_at_unix: i64 }, Error(String) }`
  - `worker::WuSearchFn = fn() -> Result<Vec<AvailableDriverUpdate>, ProbeError>`; `worker::WorkerConfig { make_probe: ProbeFactory, wu_search: WuSearchFn, last_scan_path: PathBuf }`; `worker::WorkerHandle { tx: Sender<WorkerCommand>, rx: Receiver<WorkerEvent> }`; `worker::spawn_worker(cfg: WorkerConfig, repaint: egui::Context) -> WorkerHandle`
  - `nav::Page { #[default] Dashboard, Analyzer, Live, Drivers, Games, Settings }` with `fn title(self) -> &'static str` (`"Dashboard"`, `"FPS Analyzer"`, `"Live Monitor"`, `"Drivers"`, `"Games"`, `"Settings"`); `nav::sidebar(ui: &mut egui::Ui, state: &mut AppState)`
  - `widgets::toasts::{ToastKind { Success, Info, Error }, Toast { text: String, kind: ToastKind, created: f64 }, Toasts /* Default */ { fn push(&mut self, text: impl Into<String>, kind: ToastKind, now: f64); fn visible(&self, now: f64) -> Vec<&Toast>; fn show(&mut self, ctx: &egui::Context, p: &Palette, now: f64) }}` (4.0 s lifetime)
  - `state::UiEffect { Send(WorkerCommand), OpenUrl(String), SaveSettings }` (derive `Debug, Clone, PartialEq`); `Settings` derives `Debug, Clone, PartialEq`
  - `state::AppState` with public fields `page: Page, settings: Settings, palette: Palette, scan: Option<ScanResult>, scan_step: Option<ScanStep>, findings: Vec<Finding>, health: u8, hardware: Option<HardwareProfile>, live: LiveHistory /* capacity 120 */, live_running: bool, fps: Option<FrameStats>, frame_times: Vec<f64>, fps_target: Option<String>, drivers: Vec<DriverRow>, drivers_checking: bool, drivers_checked_at: Option<i64>, catalog_source: Option<CatalogSource>, expanded_finding: Option<&'static str>, selected_game: Option<String>, driver_class_filter: Option<String>, toasts: Toasts, effects: Vec<UiEffect>, now: f64` and methods `fn new(settings: Settings) -> Self`, `fn apply_event(&mut self, e: WorkerEvent)`, `fn set_page(&mut self, p: Page)`, `fn set_theme(&mut self, t: ThemeChoice)`
  - `app::SpeedImproverApp { pub state: AppState, .. }` with `fn new(cc: &eframe::CreationContext, paths: AppPaths, make_probe: ProbeFactory) -> Self`, `fn headless(state: AppState) -> Self`, `fn show(&mut self, ui: &mut egui::Ui)`; `impl eframe::App` (`logic` drains worker events and effects; `ui` calls `show`)
  - `pages::settings::show(ui: &mut egui::Ui, state: &mut AppState)`; every later page exposes the same signature
  - `testing::state_with(snapshot: SystemSnapshot, games: Vec<InstalledGame>) -> AppState` (applies `ScanDone`, then clears `effects`); `testing::demo_games() -> Vec<InstalledGame>` (CS2 `steam:730` on `C:\…`, Cyberpunk `steam:1091500`, Fortnite `epic:Fortnite`)

Behavior that the tests pin:
- `apply_event(ScanDone)`: clears `scan_step`; recomputes `findings` (`analyze`), `health`, `hardware`; toasts `"Scan complete"` (Success); toasts `"Some data was unavailable ({n})"` (Info) when warnings exist; when `drivers_checked_at` is `None` and not checking, pushes `Send(CheckDrivers { catalog_url })` and sets `drivers_checking`.
- `set_page`: entering Dashboard/Live while `!live_running` pushes `Send(StartLive)` and sets `live_running`; moving to any other page while `live_running` pushes `Send(StopLive)` and clears it. `SpeedImproverApp::new` calls `state.set_page(Page::Dashboard)` so sampling starts at launch.
- `set_theme` updates `palette` and pushes `SaveSettings`.
- Sidebar buttons show only the icon but expose `Page::title()` as their accessibility label (`response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, title))`), with the title as hover text.
- `show()` installs fonts and applies the theme once (and again whenever `palette` changes), sets `state.now` from `ui.ctx().input(|i| i.time)`, draws the sidebar, the current page and toasts.
- Worker: owns the probe (constructed inside the thread); `Scan` emits `ScanProgress` per step then `ScanDone` and writes `last_scan_path`; live sampling and PresentMon draining run every 1 000 ms while active (`recv_timeout`); `CheckDrivers` runs `load_catalog` + `wu_search` on a separate thread, then `evaluate_drivers` on the last snapshot, emitting `Drivers` (and `Error("Windows Update check failed: …")` if the search failed); every send is followed by `repaint.request_repaint()`.
- `main.rs`: `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`; `--fixture <path>` selects `FixtureProbe`; logs to `log_file` via `tracing_subscriber::fmt().with_writer(Mutex::new(file))`; window 1440×900, min 1100×700, title `SpeedImprover`. On start the app loads `last_scan.json` (if readable) as an initial `ScanDone`, then sends `Scan`.
- Settings page: `Dark theme` checkbox, `PresentMon path` text field, hyperlink `Download PresentMon` → `https://github.com/GameTechDev/PresentMon/releases`, `Driver catalog URL` field with `Reset` button, `Rescan now` button (`Send(Scan)`). Edits push `SaveSettings`.
- `assets/demo.json` = `ScanResult` with `struggling_laptop` + `demo_games()`.

- [ ] **Step 1: Write the failing tests**

```rust
mod common;
use common::*;

#[test] fn settings_load_defaults_and_round_trip() {
    let dir = std::env::temp_dir().join(format!("si-app-{}", std::process::id()));
    let paths = AppPaths::in_dir(&dir);
    assert_eq!(Settings::load(&paths.config_file), Settings::default());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&paths.config_file, "not = [valid").unwrap();
    assert_eq!(Settings::load(&paths.config_file), Settings::default());
    let s = Settings { favorites: vec!["steam:730".into()], ..Default::default() };
    s.save(&paths.config_file).unwrap();
    assert_eq!(Settings::load(&paths.config_file), s);
}

#[test] fn scan_done_computes_findings_and_requests_drivers() {
    let mut st = AppState::new(Settings::default());
    st.apply_event(WorkerEvent::ScanDone(Box::new(si_probe::ScanResult { snapshot: fixtures::struggling_laptop(), ..Default::default() })));
    assert_eq!((st.findings.len(), st.health), (14, 0));
    assert!(st.hardware.is_some());
    assert!(st.toasts.visible(st.now).iter().any(|t| t.text == "Scan complete"));
    assert!(st.effects.contains(&UiEffect::Send(WorkerCommand::CheckDrivers { catalog_url: DEFAULT_CATALOG_URL.into() })));
}

#[test] fn live_follows_page() {
    let mut st = testing::state_with(fixtures::healthy_desktop(), vec![]);
    st.set_page(Page::Live);
    assert_eq!(st.effects.pop(), Some(UiEffect::Send(WorkerCommand::StartLive)));
    assert!(st.live_running);
    st.set_page(Page::Drivers);
    assert_eq!(st.effects.pop(), Some(UiEffect::Send(WorkerCommand::StopLive)));
}

#[test] fn sidebar_navigates() {
    let mut h = harness(testing::state_with(fixtures::healthy_desktop(), vec![]));
    h.get_by_label("Settings").click();
    h.run();
    assert_eq!(h.state().state.page, Page::Settings);
    h.get_by_label("Download PresentMon");
}

#[test] fn worker_scans_and_samples_with_fixture_probe() {
    let cfg = WorkerConfig {
        make_probe: Box::new(|| Box::new(si_probe::fixture::FixtureProbe::new(si_probe::ScanResult { snapshot: fixtures::healthy_desktop(), ..Default::default() }))),
        wu_search: || Ok(vec![]),
        last_scan_path: std::env::temp_dir().join(format!("si-last-{}.json", std::process::id())),
    };
    let w = spawn_worker(cfg, egui::Context::default());
    w.tx.send(WorkerCommand::Scan).unwrap();
    w.tx.send(WorkerCommand::StartLive).unwrap();
    w.tx.send(WorkerCommand::CheckDrivers { catalog_url: "http://127.0.0.1:9/x.json".into() }).unwrap();
    let (mut done, mut sample, mut drivers) = (false, false, false);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !(done && sample && drivers) && std::time::Instant::now() < deadline {
        match w.rx.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(WorkerEvent::ScanDone(_)) => done = true,
            Ok(WorkerEvent::Sample(_)) => sample = true,
            Ok(WorkerEvent::Drivers { catalog, .. }) => { drivers = true; assert_eq!(catalog, si_probe::catalog::CatalogSource::Bundled); }
            _ => {}
        }
    }
    assert!(done && sample && drivers);
    w.tx.send(WorkerCommand::Shutdown).unwrap();
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p si-app --test shell`.

- [ ] **Step 3: Implement the shell** (files listed above, behavior above).

- [ ] **Step 4: Verify pass** — 5 pass.

- [ ] **Step 5: Smoke-run in dev mode**

Run: `cargo run -p si-app -- --fixture crates/si-app/assets/demo.json`
Expected: window opens on the Dashboard placeholder with a sidebar; clicking icons switches pages; "Scan complete" toast appears and disappears after 4 s.

- [ ] **Step 6: Commit** — `git add Cargo.toml crates/si-app && git commit -m "feat(app): app shell with theme, worker, navigation and settings"`

### Task 15: Widgets

**Files:**
- Create: `crates/si-app/src/widgets/{card,pill,donut,chart,format}.rs`
- Test: `crates/si-app/tests/widgets.rs`

**Interfaces:**
- Consumes: `Palette`, `Severity`, `Tier`, `DriverStatus`.
- Produces:
  - `card::card(ui: &mut Ui, p: &Palette, title: &str, action: Option<&str>, add: impl FnOnce(&mut Ui)) -> CardResponse { action_clicked: bool }` (radius 14, padding 16, `card` fill, title top-left, action as a muted link top-right)
  - `card::gradient_card(ui: &mut Ui, p: &Palette, lines: &[&str])` (mesh gradient `accent_strong` → `#4C1D95`, white text, radius 14)
  - `pill::PillKind { Primary, Neutral, Warning, Muted }`; `pill::pill(ui: &mut Ui, p: &Palette, text: &str, kind: PillKind) -> Response` (radius 10)
  - `pill::status_dot(ui: &mut Ui, color: Color32)` (8 px circle)
  - `donut::donut(ui: &mut Ui, p: &Palette, pct: Option<f32>, caption: &str, size: f32) -> Response` (track in `border`, arc in `accent` starting at 12 o'clock clockwise, centre text `"73%"` or `"—"` painted and exposed once via `response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, text))`, caption below)
  - `chart::line_chart(ui: &mut Ui, id: &str, series: &[(&str, Vec<[f64; 2]>, Color32)], y_max: f64, height: f32)` (egui_plot, no grid, no axes labels, interaction disabled)
  - `format::format_mb(mb: u64) -> String`; `format::severity_color(s: Severity, p: &Palette) -> Color32`; `format::tier_badge(t: Tier) -> String` (`"HIGH TIER"`); `format::driver_status_label(s: &DriverStatus) -> &'static str` (`"Update"`, `"Check"`, `"No update found"`)

- [ ] **Step 1: Write the failing tests**

```rust
mod common;
use common::*;
use si_app::{theme::Palette, widgets::{card::*, donut::*, format::*}};

#[test] fn format_mb_cases() {
    assert_eq!(format_mb(512), "512 MB");
    assert_eq!(format_mb(8188), "8 GB");
    assert_eq!(format_mb(12282), "12 GB");
    assert_eq!(format_mb(7900), "7.7 GB");
}
#[test] fn badge_and_status_texts() {
    assert_eq!(tier_badge(si_core::tier::Tier::High), "HIGH TIER");
    assert_eq!(driver_status_label(&si_core::drivers::DriverStatus::PossiblyOutdated { age_days: 400 }), "Check");
    assert_eq!(driver_status_label(&si_core::drivers::DriverStatus::NoUpdateFound), "No update found");
}
#[test] fn card_action_is_clickable() {
    let mut h = Harness::new_ui_state(|ui, clicked: &mut bool| {
        if card(ui, &Palette::dark(), "Drivers", Some("View All"), |ui| { ui.label("row"); }).action_clicked { *clicked = true; }
    }, false);
    h.get_by_label("Drivers");
    h.get_by_label("View All").click();
    h.run();
    assert!(*h.state());
}
#[test] fn donut_shows_value_or_dash() {
    let h = Harness::new_ui(|ui| { donut(ui, &Palette::dark(), Some(73.4), "CPU", 96.0); donut(ui, &Palette::dark(), None, "GPU", 96.0); });
    h.get_by_label("73%");
    h.get_by_label("—");
}
#[test] fn toasts_expire_after_four_seconds() {
    let mut t = si_app::widgets::toasts::Toasts::default();
    t.push("Scan complete", si_app::widgets::toasts::ToastKind::Success, 10.0);
    assert_eq!(t.visible(13.9).len(), 1);
    assert_eq!(t.visible(14.1).len(), 0);
}
```

(`format_mb`: < 1024 ⇒ `"{mb} MB"`; else GB with one decimal, `.0` stripped.)

- [ ] **Step 2: Verify failure** — `cargo test -p si-app --test widgets`.
- [ ] **Step 3: Implement the widgets.**
- [ ] **Step 4: Verify pass** — 5 pass.
- [ ] **Step 5: Commit** — `git add crates/si-app && git commit -m "feat(app): card, pill, donut, chart and format widgets"`

### Task 16: Dashboard page

**Files:**
- Create: `crates/si-app/src/pages/dashboard.rs`
- Test: `crates/si-app/tests/dashboard.rs`

**Interfaces:**
- Consumes: Task 14 `AppState`, Task 15 widgets.
- Produces: `pages::dashboard::show(ui: &mut egui::Ui, state: &mut AppState)`

Layout per spec §9 table: three columns (≈ 24 % / 38 % / 38 % of the width, gap 12), page scrolls vertically. Exact copy: card titles `System Profile`, `Dark Theme`, `System Summary`, `Live`, `Drivers` (action `View All`), `Graphics Card`, `Favorite Games`, `FPS Killers` (action `Open Analyzer`); health pill `Health {n}/100`; status line `Status: Scan complete` (green dot) or the current `ScanStep::label()` while scanning; banner `{n} high-impact issues found` / `No high-impact issues found`; plus tile accessibility label `Add game`; driver card empty states `Checking for driver updates…` / `Not checked yet`; driver pills via `format::driver_status_label`. Long names use truncating labels. FPS Killers tile click ⇒ `page = Analyzer`, `expanded_finding = Some(id)`; Favorite Games tile click ⇒ `page = Games`, `selected_game = Some(key)`. Clickable tiles paint their text and expose exactly one accessible node labelled with the title (via `widget_info`), so kittest labels stay unique.

- [ ] **Step 1: Write the failing tests**

```rust
mod common;
use common::*;

fn h() -> Harness<'static, SpeedImproverApp> {
    let mut st = testing::state_with(fixtures::struggling_laptop(), testing::demo_games());
    st.settings.favorites = vec!["steam:730".into()];
    harness(st)
}

#[test] fn shows_profile_health_and_tier() {
    let h = h();
    for l in ["System Profile", "LAPTOP-STUDENT", "Health 0/100", "LOW TIER", "7 high-impact issues found", "Status: Scan complete"] { h.get_by_label(l); }
}
#[test] fn lists_fps_killers_and_opens_analyzer() {
    let mut h = h();
    h.get_by_label("Monitor running below its max refresh rate").click();
    h.run();
    assert_eq!(h.state().state.page, Page::Analyzer);
    assert_eq!(h.state().state.expanded_finding, Some("display.refresh_below_max"));
}
#[test] fn view_all_opens_drivers() {
    let mut h = h();
    h.get_by_label("Checking for driver updates…");
    h.get_by_label("View All").click();
    h.run();
    assert_eq!(h.state().state.page, Page::Drivers);
}
#[test] fn favorites_and_add_tile() {
    let mut h = h();
    h.get_by_label("Counter-Strike 2");
    h.get_by_label("Add game").click();
    h.run();
    assert_eq!(h.state().state.page, Page::Games);
}
```

(`testing::state_with` leaves `drivers_checking = true` because the ScanDone handler requested a check.)

- [ ] **Step 2: Verify failure** — `cargo test -p si-app --test dashboard`.
- [ ] **Step 3: Implement the page**; wire `Page::Dashboard` in `app.rs`.
- [ ] **Step 4: Verify pass** — 4 pass.
- [ ] **Step 5: Visual check** — `cargo run -p si-app -- --fixture crates/si-app/assets/demo.json` at 1440×900 and 1100×700: no overlapping cards, no horizontal scroll, matches the mockup's card arrangement in dark and light themes.
- [ ] **Step 6: Commit** — `git add crates/si-app && git commit -m "feat(app): dashboard page"`

### Task 17: FPS Analyzer page

**Files:**
- Create: `crates/si-app/src/pages/analyzer.rs`
- Test: `crates/si-app/tests/analyzer.rs`

**Interfaces:**
- Produces: `pages::analyzer::show(ui: &mut egui::Ui, state: &mut AppState)`

Group headings `High impact ({n})`, `Medium impact ({n})`, `Low impact ({n})` (empty groups hidden). Each row: severity dot + title; clicking the title toggles `expanded_finding`. Expanded row shows `detail` and: `Fix::OpenUri` ⇒ button `Fix` (pushes `OpenUrl(uri)`); `Fix::ShowDrivers` ⇒ button `Open Drivers` (`set_page(Drivers)`); `Fix::Manual(text)` ⇒ label `How to fix: {text}`. Header button `Rescan` pushes `Send(Scan)`. Empty state: `No FPS problems found`.

- [ ] **Step 1: Write the failing tests**

```rust
mod common;
use common::*;

fn analyzer(s: si_core::model::SystemSnapshot) -> Harness<'static, SpeedImproverApp> {
    let mut st = testing::state_with(s, vec![]);
    st.page = Page::Analyzer;
    harness(st)
}
fn h() -> Harness<'static, SpeedImproverApp> { analyzer(fixtures::struggling_laptop()) }
fn h_healthy() -> Harness<'static, SpeedImproverApp> { analyzer(fixtures::healthy_desktop()) }

#[test] fn groups_by_severity() { let h = h(); h.get_by_label("High impact (7)"); h.get_by_label("Medium impact (5)"); h.get_by_label("Low impact (2)"); }
#[test] fn fix_button_opens_settings_uri() {
    let mut h = h();
    h.get_by_label("Game Mode is off").click(); h.run();
    h.get_by_label("Fix").click(); h.run();
    assert!(h.state().state.effects.contains(&UiEffect::OpenUrl("ms-settings:gaming-gamemode".into())));
}
#[test] fn driver_fix_navigates() {
    let mut h = h();
    h.get_by_label("Graphics driver is out of date").click(); h.run();
    h.get_by_label("Open Drivers").click(); h.run();
    assert_eq!(h.state().state.page, Page::Drivers);
}
#[test] fn healthy_pc_shows_empty_state() { h_healthy().get_by_label("No FPS problems found"); }
```

(Struggling fixture: High = refresh, battery, power saver, single channel, below rated, pressure, driver age = 7; Medium = low_total, game mode, recording, low free, hogs = 5; Low = memory integrity, startup = 2.)

- [ ] **Step 2: Verify failure** — `cargo test -p si-app --test analyzer`.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Verify pass** — 4 pass.
- [ ] **Step 5: Commit** — `git add crates/si-app && git commit -m "feat(app): FPS analyzer page"`

### Task 18: Live Monitor page

**Files:**
- Create: `crates/si-app/src/pages/live.rs`
- Test: `crates/si-app/tests/live_page.rs`

**Interfaces:**
- Produces: `pages::live::show(ui: &mut egui::Ui, state: &mut AppState)`; `pages::live::default_fps_target(sample: &LiveSample) -> Option<String>`

Content: verdict banner = `state.live.verdict().explanation()`; four 120 s line charts (`CPU`, `GPU`, `RAM`, `VRAM`) with current value in the title; table `Top processes` (name, CPU %, GPU %, memory) from the latest sample; FPS panel: if `presentmon_path` is `None` ⇒ label `Set the PresentMon path in Settings to measure real FPS` + button `Open Settings`; else a `Game process` combo (defaults to `default_fps_target`), button `Measure FPS` ⇒ `Send(StartFps { presentmon, process_name })`, `Stop` ⇒ `Send(StopFps)`, labels `Avg {:.0} FPS` and `1% low {:.0} FPS` when `fps` is set, frame-time chart, and the note `Requires administrator or Performance Log Users membership`.

- [ ] **Step 1: Write the failing tests**

```rust
mod common;
use common::*;
use si_app::pages::live::default_fps_target;
use si_core::{live::*, model::ProcessUsage};

fn gpu_bound_state() -> AppState {
    let mut st = testing::state_with(fixtures::healthy_desktop(), vec![]);
    st.page = Page::Live;
    for i in 0..10 { st.live.push(LiveSample { t_ms: i * 1000, gpu_pct: Some(99.0), cpu_max_core_pct: 60.0, ram_total_mb: 32768, ram_used_mb: 9000, ..Default::default() }); }
    st
}
#[test] fn shows_verdict() { harness(gpu_bound_state()).get_by_label(Bottleneck::GpuBound.explanation()); }
#[test] fn prompts_for_presentmon() {
    let mut h = harness(gpu_bound_state());
    h.get_by_label("Open Settings").click(); h.run();
    assert_eq!(h.state().state.page, Page::Settings);
}
#[test] fn shows_fps_stats() {
    let mut st = gpu_bound_state();
    st.settings.presentmon_path = Some("C:\\Tools\\PresentMon.exe".into());
    st.fps = Some(si_core::frames::FrameStats { avg_fps: 92.6, low_1pct_fps: 20.0, frames: 100 });
    let h = harness(st);
    h.get_by_label("Avg 93 FPS");
    h.get_by_label("1% low 20 FPS");
}
#[test] fn default_target_skips_dwm() {
    let p = |n: &str, g| ProcessUsage { name: n.into(), gpu_pct: g, ..Default::default() };
    let s = LiveSample { top_processes: vec![p("dwm.exe", Some(40.0)), p("cs2.exe", Some(35.0)), p("chrome.exe", None)], ..Default::default() };
    assert_eq!(default_fps_target(&s).as_deref(), Some("cs2.exe"));
    assert_eq!(default_fps_target(&LiveSample::default()), None);
}
```

- [ ] **Step 2: Verify failure** — `cargo test -p si-app --test live_page`.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Verify pass** — 4 pass.
- [ ] **Step 5: Commit** — `git add crates/si-app && git commit -m "feat(app): live monitor page with bottleneck verdict and FPS capture"`

### Task 19: Drivers page

**Files:**
- Create: `crates/si-app/src/pages/drivers.rs`
- Test: `crates/si-app/tests/drivers_page.rs`

**Interfaces:**
- Consumes: `format::driver_status_label` (Task 15), `DriverRow`, `CatalogSource`.
- Produces: `pages::drivers::show(ui: &mut egui::Ui, state: &mut AppState)`

Content: header button `Check for updates` (disabled with spinner while `drivers_checking`) ⇒ `Send(CheckDrivers { catalog_url })` + `drivers_checking = true`; class filter combo (`All classes` + distinct classes); `Catalog: online` / `Catalog: built-in`; `Checked HH:MM` (local time via `chrono::Local`); table columns Device, Class, Version, Date, Status. Pills: `Update` (Primary) ⇒ `OpenUrl(url)`; `Check` (Warning) ⇒ `OpenUrl("ms-settings:windowsupdate-optionalupdates")`; `No update found` (Muted, not clickable).

- [ ] **Step 1: Write the failing tests**

```rust
mod common;
use common::*;
use si_core::drivers::*;

fn drivers_state() -> AppState {
    let mut st = testing::state_with(fixtures::struggling_laptop(), vec![]);
    st.page = Page::Drivers;
    st.apply_event(WorkerEvent::Drivers {
        rows: evaluate_drivers(&fixtures::struggling_laptop(), &DriverCatalog::empty(), &[]),
        catalog: si_probe::catalog::CatalogSource::Bundled, checked_at_unix: 1_793_491_200 });
    st.effects.clear();
    st
}
#[test] fn lists_rows_with_status() {
    let h = harness(drivers_state());
    h.get_by_label("Intel(R) Iris(R) Xe Graphics");
    h.get_by_label("Catalog: built-in");
    assert_eq!(h.get_all_by_label("Check").count(), 2);
}
#[test] fn check_pill_opens_optional_updates() {
    let mut h = harness(drivers_state());
    h.get_all_by_label("Check").next().unwrap().click(); h.run();
    assert!(h.state().state.effects.contains(&UiEffect::OpenUrl("ms-settings:windowsupdate-optionalupdates".into())));
}
#[test] fn check_for_updates_sends_command() {
    let mut h = harness(drivers_state());
    h.get_by_label("Check for updates").click(); h.run();
    assert!(h.state().state.drivers_checking);
    assert!(matches!(h.state().state.effects.last(), Some(UiEffect::Send(WorkerCommand::CheckDrivers { .. }))));
}
```

- [ ] **Step 2: Verify failure** — `cargo test -p si-app --test drivers_page`.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Verify pass** — 3 pass.
- [ ] **Step 5: Commit** — `git add crates/si-app && git commit -m "feat(app): drivers page"`

### Task 20: Games page

**Files:**
- Create: `crates/si-app/src/pages/games.rs`
- Test: `crates/si-app/tests/games_page.rs`

**Interfaces:**
- Produces: `pages::games::show(ui: &mut egui::Ui, state: &mut AppState)`; `pages::games::reason_text(r: &Reason) -> String`

Content: header `Your PC: {overall} tier`; left list of `scan.games` (selectable by name; star button with accessibility label `Favorite {name}` toggles `"{store}:{id}"` in `settings.favorites` and pushes `SaveSettings`); right: goal selectors `Competitive` / `Balanced` / `Quality` (change ⇒ `settings.goal` + `SaveSettings`); tile grid of `recommend(profile, hardware, goal)` — label, value, impact dot, `reason_text` on hover. Empty states: `No Steam or Epic games found`; `No profile yet for {name}`. Default selection = first game with a profile.

`reason_text`: `Tier(t)` ⇒ `Matches your {t.label()}-tier PC`; `VramCap(mb)` ⇒ `Limited by {format_mb(mb)} of VRAM`; `Competitive` ⇒ `Competitive: favors FPS`; `QualityBoost` ⇒ `Cheap on FPS, raised for quality`.

- [ ] **Step 1: Write the failing tests**

```rust
mod common;
use common::*;
use si_app::pages::games::reason_text;
use si_core::games::{profile::ProfileLibrary, recommend::*};

fn games_state() -> AppState {
    let mut st = testing::state_with(fixtures::healthy_desktop(), testing::demo_games());
    st.page = Page::Games;
    st
}
#[test] fn shows_recommendations_for_first_profiled_game() {
    let h = harness(games_state());
    h.get_by_label("Your PC: High tier");
    let cs2 = ProfileLibrary::bundled().profiles.into_iter().find(|p| p.id == "cs2").unwrap();
    h.get_by_label(&cs2.settings[0].label);
}
#[test] fn goal_change_is_saved() {
    let mut h = harness(games_state());
    h.get_by_label("Competitive").click(); h.run();
    assert_eq!(h.state().state.settings.goal, Goal::Competitive);
    assert!(h.state().state.effects.contains(&UiEffect::SaveSettings));
}
#[test] fn star_toggles_favorite() {
    let mut h = harness(games_state());
    h.get_by_label("Favorite Counter-Strike 2").click(); h.run();
    assert_eq!(h.state().state.settings.favorites, ["steam:730"]);
}
#[test] fn reason_texts() {
    assert_eq!(reason_text(&Reason::VramCap(6144)), "Limited by 6 GB of VRAM");
    assert_eq!(reason_text(&Reason::Tier(si_core::tier::Tier::High)), "Matches your High-tier PC");
}
#[test] fn no_games_empty_state() {
    let mut st = testing::state_with(fixtures::healthy_desktop(), vec![]);
    st.page = Page::Games;
    harness(st).get_by_label("No Steam or Epic games found");
}
```

- [ ] **Step 2: Verify failure** — `cargo test -p si-app --test games_page`.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Verify pass** — 5 pass. Then `cargo test --workspace` → everything passes.
- [ ] **Step 5: Commit** — `git add crates/si-app && git commit -m "feat(app): games page with per-game settings recommendations"`

---

## Phase 4 — Release

### Task 21: Packaging, release workflow and real-hardware verification

**Files:**
- Create: `crates/si-app/build.rs`, `crates/si-app/assets/icon.ico`, `README.md`, `.github/workflows/release.yml`
- Modify: `Cargo.toml` (`[profile.release] lto = "thin"`, `codegen-units = 1`, `strip = true`), `crates/si-app/Cargo.toml` (`[target.'cfg(windows)'.build-dependencies] winresource = "0.1.34"`)

**Interfaces:**
- Consumes: the finished app.
- Produces: `speedimprover.exe` with icon, version info (`ProductName` = `SpeedImprover`) and an `asInvoker` manifest; a release zip on tags `v*`.

- [ ] **Step 1: `build.rs`** — on Windows only, `winresource::WindowsResource::new().set_icon("assets/icon.ico").set("ProductName", "SpeedImprover").set_manifest(<asInvoker + PerMonitorV2 DPI awareness manifest>).compile()`. Icon: purple (`#8B5CF6`) rounded square with a white gauge glyph, 16/32/48/256 px.

- [ ] **Step 2: Release workflow** — on `push: tags: ['v*']`, `windows-latest`: `cargo build --release -p si-app`, zip `target/release/speedimprover.exe` + `README.md` as `SpeedImprover-${{ github.ref_name }}-x64.zip`, attach to a GitHub release (`softprops/action-gh-release@v2`).

- [ ] **Step 3: README** — what it does, screenshots, build (`cargo build --release -p si-app`), dev mode (`cargo run -p si-app -- --fixture crates/si-app/assets/demo.json`), how to enable FPS capture (PresentMon path, admin note), how to update `data/driver_catalog.json` and add a game profile (`data/games/<id>.toml`, schema from spec §8), privacy statement (no telemetry; only the catalog URL and Windows Update are contacted).

- [ ] **Step 4: Verify the build** — on Windows: `cargo build --release -p si-app` succeeds; `speedimprover.exe` shows the icon in Explorer and starts without a console window or UAC prompt.

- [ ] **Step 5: Real-hardware checklist** (run on at least one desktop with a discrete GPU and one laptop; record results in the PR):
  - Scan finishes in < 10 s; no panics in `speedimprover.log`.
  - Set the monitor to 60 Hz ⇒ `Monitor running below its max refresh rate` appears; set it back ⇒ gone after `Rescan`.
  - Toggle Game Mode off ⇒ finding appears; `Fix` opens the Game Mode settings page.
  - Laptop on battery ⇒ `Laptop is running on battery`; hybrid laptop ⇒ tier uses the dGPU.
  - Run a game: Live Monitor shows GPU % from PDH (and temperature on NVIDIA); verdict changes between GPU-bound (uncapped, max settings) and FrameCapped (60 FPS cap).
  - PresentMon path set, run as administrator ⇒ `Measure FPS` shows avg / 1 % low matching the in-game counter within ±5 %.
  - Drivers page: `Check for updates` finishes, rows show statuses, `Update` opens the vendor page.
  - Games page lists Steam and Epic games; profile recommendations change with the goal.
  - Non-English Windows (if available, or a VM with another display language): `cpu_perf_pct` and GPU % are present.

- [ ] **Step 6: Commit** — `git add . && git commit -m "build: Windows resources, release workflow and README"`
