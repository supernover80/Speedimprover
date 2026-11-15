# SpeedImprover — Pro Upgrade Design

**Date:** 2026-11-15 · **Status:** Approved by the user ("just do what you think is the best")
**Extends:** `2026-11-14-speedimprover-design.md`. Where this document and that one differ, this one wins.

## 1. What the user asked for

1. Show CPU and GPU temperatures.
2. Show *exactly* what is bottlenecking the frame rate.
3. Install driver updates automatically.
4. Look like a professional system tool, not a game UI.

## 2. Decisions (made by Claude; the user delegated them)

| Topic | Decision | Why |
|---|---|---|
| GPU temperature | NVML (NVIDIA) → LibreHardwareMonitor → Windows' `D3DKMT` adapter performance data (`KMTQAITYPE_ADAPTERPERFDATA`, the source Task Manager uses; `Temperature` is in tenths of °C, 0 = not supported) | Works on every GPU vendor with a WDDM 2.4+ driver, no admin rights |
| CPU temperature | LibreHardwareMonitor / OpenHardwareMonitor WMI (`root\LibreHardwareMonitor`, class `Sensor`) when running → PDH `\Thermal Zone Information(*)\Temperature` (Kelvin, hottest zone) labelled **approximate** → none | Exact CPU temperature needs a kernel driver; we do not ship one |
| Bottleneck | Per-frame analysis from PresentMon's `CPUBusy` / `GPUBusy` columns when FPS capture runs; otherwise the existing load-based classifier. Both produce a report with a headline, the evidence (numbers), and actions | "Exact" = measured per frame, with evidence shown |
| Driver install | One click through **Windows Update** only: per-driver **Install** and **Install all**, confirmation dialog, then the app relaunches itself elevated (UAC) to create a **System Restore point** and download + install the selected updates through the Windows Update Agent API | Microsoft-tested drivers; nothing downloaded from arbitrary sites; the elevated helper can only install updates Windows Update itself offers (matched by update ID) |
| Vendor-only GPU updates | Button **Download** opens the vendor's official page (unchanged behaviour) | Silent vendor installers are fragile and riskier |
| Look | Neutral Windows-style palette (slate greys, one blue accent), 8 px corners, labelled sidebar, data tables instead of tiles, no gradients/avatars/rings, Segoe UI on Windows, theme follows the system by default | Professional system-utility look |

## 3. Behaviour details

### Temperatures
- `LiveSample` gains `cpu_temp_c: Option<f32>` and `cpu_temp_exact: bool` (true only from LibreHardwareMonitor/OpenHardwareMonitor). `gpu_temp_c` exists.
- LibreHardwareMonitor sensor choice: CPU = identifier starting `/intelcpu` or `/amdcpu`, type `Temperature`; prefer names `CPU Package`, `Core (Tctl/Tdie)`, `Tctl/Tdie`, `Package`, else the maximum. GPU = identifier starting `/gpu-nvidia`, `/gpu-amd` or `/gpu-intel`, type `Temperature`; prefer `GPU Core`, else the maximum.
- Thermal zones: hottest zone, Kelvin − 273.15; readings ≤ 0 °C or > 125 °C ignored.
- WMI for LibreHardwareMonitor is retried at most every 30 s when absent.
- UI shows `—` when unknown and `(approx.)` for thermal-zone CPU readings.

### Bottleneck report (`si_core::bottleneck`)
- Inputs: the last 10 live samples, the PresentMon frames of the last 5 s, the display's max refresh rate.
- **Frame mode** (≥ 30 frames with both busy values): `gpu_share = mean(GPUBusy)/mean(frame)`, `cpu_share = mean(CPUBusy)/mean(frame)`.
  - Telemetry problems first (same thresholds as before): VRAM ≥ 95 % → VramLimited; RAM ≥ 92 % → RamLimited; `% Processor Performance` < 80 with CPU ≥ 50 % → CpuThrottled; NVML thermal throttle in ≥ half the samples → GpuThrottled.
  - then `gpu_share ≥ 0.90` → GpuBound; `cpu_share ≥ 0.90` and `gpu_share < 0.85` → CpuBound; both < 0.85 → FrameCapped (if mean FPS is within 3 % of the display refresh: "locked to your monitor's N Hz refresh rate"); else Balanced.
- **Load mode**: the existing `classify`.
- Report: `headline`, `source` (frames n / samples n), `evidence` (label, value text, optional 0–1 share bar, `limiting` flag), `actions` (2–3 lines). Exact copy is in the plan.

### Driver install
- `AvailableDriverUpdate.update_id` (Windows Update GUID). `DriverRow.install_id = Some(update_id)` when the row's status came from Windows Update.
- Pill labels: Windows Update → **Install**; catalog → **Download**; possibly outdated → **Check**; else **No update found**.
- Elevated helper: `speedimprover.exe --install-drivers <result.json> <update-guid>…`. Arguments are rejected unless every ID is a GUID (`8-4-4-4-12` hex). The helper writes an `InstallReport { installed, failed, reboot_required, restore_point_created }` as JSON and exits; the app waits for it (`ShellExecuteExW` with `runas`, `WaitForSingleObject`). A declined UAC prompt → "Installation cancelled".
- After an install the app re-checks drivers; when `reboot_required`, a banner says "Restart your PC to finish installing drivers."

### Look
- Pages: Overview, Issues, Performance, Drivers, Games, Settings (sidebar shows icon + name).
- Overview: four summary tiles (Health, Bottleneck, CPU, GPU), "Performance issues" table, "System" details, "Drivers" summary with **Install all**, "Favorite games" list.
- Settings: Theme = System / Light / Dark (replaces the dashboard theme card).

## 4. Out of scope
Kernel drivers, vendor installer downloads, automatic installs without a click, fan control, overclocking.
