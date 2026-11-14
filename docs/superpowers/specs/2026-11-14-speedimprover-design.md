# SpeedImprover — Design Spec

**Date:** 2026-11-14
**Status:** Draft for review

## 1. What it is

A Windows desktop app written in Rust that:

1. **Scans the PC**: CPU, GPU(s), RAM modules, disks, displays, OS gaming settings, power state, background processes, installed drivers.
2. **Shows what is hurting FPS**: a ranked list of "FPS killers" with severity, a plain-English explanation and a one-click "Fix" that opens the right Windows Settings page. A **Live Monitor** classifies the bottleneck (CPU-bound, GPU-bound, VRAM, RAM, thermal) while a game runs, and optionally shows real FPS using PresentMon.
3. **Shows which drivers need updating**: GPU, chipset, network, audio, storage and Bluetooth drivers, checked against Windows Update and a version catalog, with a link to the vendor's official download page.
4. **Recommends in-game settings**: detects installed Steam and Epic games and, for games that have a profile, shows the best value for each setting based on the PC's hardware tier and the player's goal (Competitive / Balanced / Quality).

The UI follows the supplied mockup: dark theme, left icon sidebar, rounded cards in three columns, purple accent, donut rings and a line chart, pill buttons, status dots, toasts.

## 2. Decisions

| Topic | Decision |
|---|---|
| Target OS | Windows 10 22H2+ and Windows 11, x64. Core logic and UI also build and test on Linux/macOS (with partial data) so CI and UI work don't need Windows. |
| UI toolkit | `eframe`/`egui` 0.36 (pure Rust, single binary, immediate mode suits live stats). `egui_plot` for charts, `egui-phosphor` for the outline icon set. |
| Privileges | Runs as a normal user (`asInvoker`). Anything that needs admin degrades to "not available". PresentMon FPS capture needs admin or the *Performance Log Users* group; the UI says so. |
| Changes to the system | **Read-only.** The app never edits the registry, installs drivers or edits game config files. Fixes open `ms-settings:` / `windowsdefender:` URIs or show manual steps. Driver "Update" opens the vendor's official page or Windows Update. |
| Network | Only (a) fetching `driver_catalog.json` from this repo's GitHub raw URL (configurable, bundled copy as fallback) and (b) the OS Windows Update Agent API. No telemetry. |
| CPU temperature | Not collected in v1 (needs a kernel driver). Thermal throttling is detected from `% Processor Performance` instead. GPU temperature comes from NVML on NVIDIA only. |
| Game settings | Displayed only, never written. Profiles are data files (`data/games/*.toml`), so adding a game needs no code. |

## 3. Architecture

Cargo workspace with three crates:

```
crates/
  si-core/   pure logic, no OS calls: data model, FPS rules, health score,
             bottleneck classifier, PresentMon CSV parsing, driver status,
             hardware tiers, game library parsing, settings recommendations
  si-probe/  data collection: SystemProbe trait; sysinfo collector (all OSes);
             WMI / registry / DXGI / PDH / NVML / Windows Update collectors
             (cfg(windows)); FixtureProbe (loads snapshot JSON); PresentMon runner
  si-app/    eframe binary "speedimprover": theme, widgets, pages, worker thread
data/        driver_catalog.json, gpu_tiers.toml, cpu_tiers.toml, games/*.toml
             (embedded with include_str!; catalog also fetched at runtime)
```

**Threading.** The UI thread never blocks. A worker thread owns the probe (WMI/COM objects are not `Send`, so the probe is *constructed inside* the worker). The UI sends `WorkerCommand`s and drains `WorkerEvent`s each frame through `crossbeam-channel`; the worker calls `egui::Context::request_repaint()` after each event. The Windows Update search (10–60 s) runs on its own short-lived thread so live sampling keeps going.

**Fault tolerance.** Each collector is independent. A failing collector leaves its fields `None`/empty and adds a `ScanWarning`; a scan never aborts. Rules treat `None` as "unknown" and produce no finding.

**Dev mode.** `speedimprover --fixture <file.json>` runs the whole UI on a saved snapshot, on any OS.

## 4. Data sources (Windows)

| Data | Source |
|---|---|
| CPU name, cores, threads, base clock | WMI `Win32_Processor` (`Name`, `NumberOfCores`, `NumberOfLogicalProcessors`, `MaxClockSpeed`) |
| GPU list, vendor, dedicated VRAM | DXGI `EnumAdapters1` + `GetDesc1`; skip `DXGI_ADAPTER_FLAG_SOFTWARE` and vendor `0x1414` (Microsoft). Vendor IDs: `0x10DE` NVIDIA, `0x1002` AMD, `0x8086` Intel. (`Win32_VideoController.AdapterRAM` caps at 4 GB — don't use it for VRAM.) |
| GPU integrated? | D3D12 `CheckFeatureSupport(D3D12_FEATURE_ARCHITECTURE).UMA`; if device creation fails, name heuristic (`UHD`, `Iris`, `Radeon(TM) Graphics`, `Radeon 7xxM/8xxM`, `Vega`) |
| GPU driver version + date | WMI `Win32_VideoController` (`Name`, `DriverVersion`, `DriverDate`), joined to the DXGI adapter by name |
| RAM total / used | `sysinfo` |
| RAM modules | WMI `Win32_PhysicalMemory` (`Capacity`, `Speed`, `ConfiguredClockSpeed`, `SMBIOSMemoryType`: 26 = DDR4, 34 = DDR5) |
| Disks | `sysinfo` disks (mount, total, free) + WMI `root\Microsoft\Windows\Storage`: `MSFT_PhysicalDisk` (`DeviceId`, `MediaType` 3 = HDD, 4 = SSD; `BusType` 17 = NVMe), `MSFT_Partition` (`DiskNumber`, `DriveLetter`) |
| Displays | `EnumDisplayDevicesW` + `EnumDisplaySettingsW(ENUM_CURRENT_SETTINGS)` for current size/Hz; max Hz = highest `dmDisplayFrequency` among modes with the current width × height |
| OS name/build | Registry `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion` (`CurrentBuildNumber`, `DisplayVersion`). Build ≥ 22000 ⇒ "Windows 11" (`ProductName` still says 10). |
| Laptop / on battery | `GetSystemPowerStatus`: `BatteryFlag == 128` ⇒ no battery (desktop); `ACLineStatus == 0` ⇒ on battery |
| Power plan | `PowerGetActiveScheme` GUID: `381b4222-f694-41f0-9685-ff5bb260df2e` Balanced, `8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c` High performance, `a1841308-3541-4fab-bc81-f71556f20b4a` Power saver, `e9a42b02-d5df-448d-aa00-03f14749eb61` Ultimate. Identify by GUID, never by (localized) name. |
| Game Mode | `HKCU\Software\Microsoft\GameBar\AutoGameModeEnabled` (DWORD); missing ⇒ on (Windows default) |
| Background recording | `HKCU\Software\Microsoft\Windows\CurrentVersion\GameDVR\HistoricalCaptureEnabled` = 1 ⇒ on; missing ⇒ off |
| HAGS | `HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers\HwSchMode`: 2 ⇒ on, 1 ⇒ off, missing ⇒ unknown |
| Memory Integrity | WMI `root\Microsoft\Windows\DeviceGuard` `Win32_DeviceGuard.SecurityServicesRunning` contains 2 ⇒ on |
| Startup apps | Count values in `HKCU` and `HKLM` `…\CurrentVersion\Run`, minus values whose `…\Explorer\StartupApproved\Run` entry has an odd first byte (`0x03`, `0x07` = disabled) |
| Processes | `sysinfo` (two refreshes ≥ `MINIMUM_CPU_UPDATE_INTERVAL` apart); CPU % divided by logical core count so 100 % = whole CPU |
| Drivers | WMI `Win32_PnPSignedDriver` (`DeviceName`, `DeviceClass`, `Manufacturer`, `DriverVersion`, `DriverDate`, `HardWareID`) |
| Available driver updates | Windows Update Agent COM: `UpdateSession` → `CreateUpdateSearcher` → `Search("IsInstalled=0 and Type='Driver'")` → `IWindowsDriverUpdate` (`DriverHardwareID`, `DriverClass`, `DriverModel`, `DriverVerDate`) |
| Live GPU % / VRAM | PDH via **`PdhAddEnglishCounterW`** (counter names are localized otherwise): `\GPU Engine(*engtype_3D)\Utilization Percentage` (instances contain `pid_<n>` and `luid_…`), `\GPU Adapter Memory(*)\Dedicated Usage` |
| Live CPU throttle | PDH `\Processor Information(_Total)\% Processor Performance` and `\Processor Information(_Total)\% Processor Utility` |
| NVIDIA extras | NVML (`nvml-wrapper`): temperature, utilization, memory, throttle reasons. "Throttling" = HW slowdown, HW thermal slowdown or SW thermal slowdown; SW power cap is normal under full load and is ignored. Missing `nvml.dll` ⇒ silently skip. |
| Real FPS | PresentMon 2.x CLI (user-supplied path): `--process_name <exe> --output_stdout --no_console_stats --stop_existing_session --session_name SpeedImprover --terminate_on_proc_exit`; frame time column `MsBetweenPresents` (v1 metrics) or `FrameTime` (v2) |
| Steam games | Steam path `HKCU\Software\Valve\Steam\SteamPath` → `steamapps\libraryfolders.vdf` → each library's `steamapps\appmanifest_*.acf` (`appid`, `name`, `installdir`, `SizeOnDisk`). Skip appid 228980 (redistributables). |
| Epic games | `%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests\*.item` JSON (`DisplayName`, `InstallLocation`, `AppName`, `InstallSize`, `bIsIncompleteInstall`) |

## 5. FPS rules

Every rule is a pure function of the snapshot (and installed games). Driver age is measured from `snapshot.taken_at_unix`, never from "now", so results are reproducible.

| Rule id | Title (exact UI copy) | Condition | Severity | Fix |
|---|---|---|---|---|
| `display.refresh_below_max` | Monitor running below its max refresh rate | any display with `max_hz - current_hz >= 3` | High | `ms-settings:display-advanced` |
| `power.on_battery` | Laptop is running on battery | laptop and on battery | High | Manual: "Plug in your charger" |
| `power.plan_saver` | Power saver plan is active | power plan is Power saver | High | `ms-settings:powersleep` |
| `memory.single_channel` | RAM is running in single-channel mode | exactly 1 RAM module | High if no discrete GPU, else Medium | Manual: "Install a second matching RAM stick" |
| `memory.below_rated` | RAM is running below its rated speed | any module `configured_mhz < rated_mhz` | High | Manual: "Enable XMP/EXPO in BIOS" |
| `memory.jedec_default` | RAM may be running at default speed | desktop, not `below_rated`, DDR4 with configured ≤ 2666 or DDR5 ≤ 4800 | Low | Manual: XMP/EXPO "if your kit is rated higher" |
| `memory.pressure` | RAM is almost full | used/total ≥ 0.85 ⇒ High; ≥ 0.75 ⇒ Medium | — | Manual: close apps listed |
| `memory.low_total` | Not enough RAM for modern games | total < 7 500 MB ⇒ High; < 15 000 MB ⇒ Medium | — | Manual: upgrade to 16 GB |
| `os.game_mode_off` | Game Mode is off | Game Mode off | Medium | `ms-settings:gaming-gamemode` |
| `os.background_recording_on` | Background recording is on | background recording on | Medium | `ms-settings:gaming-gamedvr` |
| `os.hags_off` | Hardware-accelerated GPU scheduling is off | HAGS off | Low | `ms-settings:display-advancedgraphics` |
| `os.memory_integrity_on` | Memory Integrity is on | Memory Integrity on | Low; detail must say it is a security feature and turning it off is a trade-off | `windowsdefender://coreisolation/` |
| `gpu.driver_age` | Graphics driver is out of date | primary GPU driver date > 365 days old ⇒ High; > 180 ⇒ Medium | — | Opens Drivers page |
| `storage.low_free` | System drive is almost full | system drive (`C:\`) free < 10 % or < 20 GB | Medium | `ms-settings:storagesense` |
| `storage.games_on_hdd` | Games installed on a hard drive | any installed game whose install dir is on an HDD (longest matching mount prefix, case-insensitive) | Medium | Manual: move to SSD |
| `process.hogs` | Background apps are using resources | processes (not on ignore list) with CPU ≥ 10 % or memory ≥ 1 500 MB; one finding whose detail names the top 3 by CPU | Medium | Manual: close them |
| `startup.too_many` | Too many startup apps | > 15 enabled startup apps | Low | `ms-settings:startupapps` |

A total of `0` (RAM, disk) means "unknown", not "empty": rules skip it.

Process ignore list (case-insensitive): `System`, `System Idle Process`, `Idle`, `Registry`, `Memory Compression`, `MemCompression`, `dwm.exe`, `explorer.exe`, `csrss.exe`, `speedimprover.exe`.

Findings are sorted by severity (High first), then by id. **Health score** = `100 − 15·High − 8·Medium − 3·Low`, floored at 0.

## 6. Live bottleneck classifier

Samples every 1 000 ms; the UI keeps the last 120. The verdict uses the last 10 samples (fewer than 5 ⇒ `InsufficientData`). Averages are compared in this priority order — first match wins:

1. `VramLimited`: VRAM used ≥ 95 % of VRAM total
2. `RamLimited`: RAM used ≥ 92 % of RAM total
3. `CpuThrottled`: `% Processor Performance` < 80 while CPU total ≥ 50 %
4. `GpuThrottled`: NVML reports thermal throttling in ≥ half the samples
5. `GpuBound`: GPU ≥ 95 %
6. `CpuBound`: GPU < 80 % and (busiest core ≥ 90 % or CPU total ≥ 90 %)
7. `FrameCapped`: GPU < 80 % and busiest core < 90 % (V-Sync / frame cap / menu)
8. `Balanced`: anything else

Missing metrics (`None`) make their condition false. FPS stats from PresentMon: average FPS = 1000 / mean frame time; 1 % low = 1000 / 99th-percentile frame time (nearest-rank), over the last 5 s of frames.

## 7. Drivers

Shown classes: `Display`, `Net`, `Media`, `SCSIAdapter`, `HDC`, `Bluetooth`, and `System` only when the manufacturer contains `Intel` or `Advanced Micro Devices`/`AMD` (chipset).

Status, in order:

1. **Update available (Windows Update)** — a Windows Update driver's hardware ID equals (case-insensitive) one of the device's hardware IDs. Button opens `ms-settings:windowsupdate-optionalupdates`.
2. **Update available (catalog)** — catalog has an entry for the device's vendor + class and `installed < latest` by numeric dotted comparison. NVIDIA versions are compared in NVIDIA's format: Windows `32.0.15.6094` → `560.94` (digits of the last two fields, last 5 digits, dot before the final 2). Button opens the catalog `download_url`.
3. **Possibly outdated** — driver date > 365 days before scan time.
4. **No update found** — otherwise. (Never claim "up to date" without evidence.)

Unparseable versions ⇒ step 2 is skipped for that driver.

`data/driver_catalog.json` schema:

```json
{ "schema": 1, "updated": "YYYY-MM-DD",
  "entries": [ { "vendor": "nvidia", "class": "Display", "latest_version": "<nvidia-format>",
                 "download_url": "https://www.nvidia.com/en-us/drivers/" } ] }
```

Vendors: `nvidia`, `amd`, `intel`. For `amd` and `intel`, `latest_version` uses the Windows driver version format (e.g. `32.0.21025.1024`). Vendor of a driver is derived from `Manufacturer` (contains `NVIDIA` / `Advanced Micro Devices` or `AMD` / `Intel`). The maintainer updates the catalog's `latest_version` values from vendor sites; the app fetches it from `https://raw.githubusercontent.com/supernover80/speedimprover/main/data/driver_catalog.json` (configurable) with a 10 s timeout, falling back to the copy compiled into the binary.

## 8. Hardware tiers and game settings

`Tier` = `Low < Mid < High < Ultra`.

**GPU tier** — `data/gpu_tiers.toml` patterns, case-insensitive substring match; the **highest** tier among matching patterns wins. Then step down one tier if the name contains `Laptop` or has a token like `6800M` / `7600S` (3–4 digits followed by `M` or `S`). No match: integrated ⇒ Low; discrete ⇒ ≥ 12 GiB VRAM High, ≥ 8 GiB Mid, else Low (fallback never gives Ultra). Primary GPU = the discrete GPU with the most VRAM, else the first GPU.

Starter table:

- **ultra**: RTX 5090, RTX 5080, RTX 5070 Ti, RTX 4090, RTX 4080, RTX 4070 Ti, RX 9070 XT, RX 7900 XT
- **high**: RTX 5070, RTX 5060 Ti, RTX 4070, RTX 3090, RTX 3080, RTX 3070, RX 9070, RX 9060 XT, RX 7800 XT, RX 7700 XT, RX 6950 XT, RX 6900 XT, RX 6800, Arc B580
- **mid**: RTX 5060, RTX 4060, RTX 3060, RTX 2080, RTX 2070, GTX 1080 Ti, RX 7600, RX 6700 XT, RX 6650 XT, RX 6600, RX 5700, Arc A770, Arc A750, Arc B570
- **low**: RTX 3050, RTX 2060, GTX 1660, GTX 1650, GTX 1080, GTX 1070, GTX 1060, RX 6500 XT, RX 580, RX 570, Arc A380, Radeon 780M, Radeon 760M, Radeon 890M, Iris Xe, UHD Graphics, Vega

**CPU tier** — `data/cpu_tiers.toml`, same matching. Step down one tier if a token ends in `U` after digits (e.g. `i5-1235U`). No match: ≥ 8 physical cores ⇒ Mid, else Low.

- **ultra**: X3D, i9-13, i9-14, Ultra 9, Ryzen 9 7, Ryzen 9 9
- **high**: i9-12, i7-12, i7-13, i7-14, Ultra 7, Ryzen 7 7, Ryzen 7 9, Ryzen 9 5
- **mid**: i9-10, i9-11, i7-10, i7-11, i5-12, i5-13, i5-14, Ultra 5, Ryzen 5 7, Ryzen 5 9, Ryzen 7 5, Ryzen 5 5

**Overall tier** = GPU tier, minus one if the CPU tier is ≥ 2 below the GPU tier; then capped at Mid if RAM < 15 000 MB and set to Low if RAM < 7 500 MB.

**Game profile** (`data/games/<id>.toml`):

```toml
id = "cs2"
name = "Counter-Strike 2"
steam_appid = 730          # optional
epic_app_name = ""         # optional; "" or absent = none

[[settings]]
label = "Global Shadow Quality"   # exactly as in the game's menu
impact = "high"                   # FPS cost of raising it: low | medium | high
vram_sensitive = false
low = "Low"
mid = "Medium"
high = "High"
ultra = "High"
competitive = "Low"               # optional override for the Competitive goal
```

Starter profiles: Counter-Strike 2 (730), Dota 2 (570), Apex Legends (1172470), PUBG: BATTLEGROUNDS (578080), Cyberpunk 2077 (1091500), ELDEN RING (1245620), Baldur's Gate 3 (1086940), Fortnite (Epic `Fortnite`). Setting labels must match each game's current menu.

**Recommendation per setting:**

1. Start at the overall tier.
2. Goal: Competitive ⇒ one tier down; Balanced ⇒ unchanged; Quality ⇒ one tier up only for `impact = "low"` settings.
3. If `vram_sensitive`: cap by VRAM — < 4 096 MB ⇒ Low, < 6 144 ⇒ Mid, < 8 192 ⇒ High.
4. If goal is Competitive and `competitive` is set, use that value; otherwise the value for the final tier.
5. Attach the reason: `Tier(t)`, `VramCap(mb)`, `Competitive`, or `QualityBoost`. Precedence: Competitive override > VramCap (when it lowered the tier) > QualityBoost > Tier.

## 9. UI

**Window** 1440 × 900 default, 1100 × 700 minimum, title "SpeedImprover".

**Theme tokens**

| Token | Dark | Light |
|---|---|---|
| `bg` | `#0F0F14` | `#F4F4F8` |
| `card` | `#17171F` | `#FFFFFF` |
| `card_raised` | `#1F1F2A` | `#EEEEF4` |
| `border` | `#2A2A36` | `#DCDCE6` |
| `text` | `#ECECF1` | `#15151C` |
| `text_muted` | `#9A9AAE` | `#5C5C70` |
| `accent` | `#A78BFA` | `#7C3AED` |
| `accent_strong` | `#8B5CF6` | `#6D28D9` |
| `success` | `#22C55E` | `#16A34A` |
| `warning` | `#F59E0B` | `#D97706` |
| `danger` | `#EF4444` | `#DC2626` |

Card corner radius 14, pill radius 10, card padding 16, gap 12. Severity colors: High = `danger`, Medium = `warning`, Low = `accent`.

**Sidebar** (64 px, icons only, tooltip on hover, active icon in `accent`): Dashboard (`HOUSE`), FPS Analyzer (`GAUGE`), Live Monitor (`PULSE`), Drivers (`CPU`), Games (`GAME_CONTROLLER`), Settings (`GEAR`).

**Dashboard** — maps the mockup one-to-one:

| Mockup | SpeedImprover |
|---|---|
| Profile card (avatar, name, "Premium" badge, balance pill) | **System Profile**: monitor icon in accent circle, hostname, `CPU · GPU` subtitle, tier badge (e.g. `HIGH TIER`), pill `Health 82/100` |
| Dark Theme toggle card | **Dark Theme** toggle (persisted) |
| Account Summary | **System Summary**: CPU, GPU, RAM (`32 GB · 2 sticks · 6000 MT/s`), System drive (`NVMe · 412 GB free`) |
| Chart card (two rings + line) | **Live**: CPU % and GPU % donut rings + 60 s line chart |
| Reports list with Download pills | **Drivers**: up to 7 rows (icon, device, `version · date`), status pill; `View All` → Drivers page |
| Purple gradient bank card | **Graphics Card**: gradient card with GPU name, VRAM and driver version |
| Favorite Beneficiaries tiles with + | **Favorite Games**: up to 4 tiles (name, store) and a `+` tile → Games page |
| Loan Profile month grid with green checks | **FPS Killers**: `Status: Scan complete` with green dot; grid of finding tiles (title, severity dot) — up to 12 |
| Notification banner + success toast | Banner `N high-impact issues found`; toast `Scan complete` |

**Other pages**

- **FPS Analyzer**: all findings, grouped by severity; each row expands to show detail and a `Fix` button (`OpenUri`) or the manual steps.
- **Live Monitor**: Start/Stop; bottleneck verdict banner with one-line explanation; CPU, GPU, RAM, VRAM charts (120 s); top 8 processes by CPU and by GPU; FPS panel (avg, 1 % low, frame-time chart) when a PresentMon path is set and a target process is chosen (default: highest GPU % process, excluding `dwm.exe`).
- **Drivers**: full table, filter by class, `Check Windows Update` button with spinner, status pill per row, last-checked time.
- **Games**: installed games list (star toggles favorite); goal selector Competitive / Balanced / Quality; recommended settings as a tile grid (label, value, impact dot, reason on hover). Games without a profile show "No profile yet".
- **Settings**: theme, PresentMon path (with "Download PresentMon" link to `https://github.com/GameTechDev/PresentMon/releases`), driver catalog URL, "Rescan".

**Toasts** bottom-right, auto-dismiss after 4 s.

**Persistence** (`directories::ProjectDirs::from("com", "SpeedImprover", "SpeedImprover")`): `config_dir/settings.toml` (theme, favorites as `steam:730` / `epic:Fortnite`, goal, PresentMon path, catalog URL); `data_dir/last_scan.json` (shown instantly on next launch, then a fresh scan starts); `data_dir/speedimprover.log`. Unreadable files ⇒ defaults, logged.

## 10. Out of scope (v1)

Applying fixes automatically, installing drivers, writing game configs, CPU temperatures, GOG/Xbox/Battle.net/Riot libraries, AMD/Intel GPU telemetry beyond PDH, overclocking, installers beyond a zipped release.
