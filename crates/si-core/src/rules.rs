//! The FPS rules (spec §5). Each rule is a pure function of the snapshot and
//! returns at most one finding. Unknown values (`None`, `0`) never trigger a rule.

use crate::findings::{Category, Finding, Fix, Severity};
use crate::model::{DiskInfo, DiskMedia, InstalledGame, MemoryGen, PowerPlan, SystemSnapshot};

/// Processes that are part of Windows (or this app) and never count as hogs.
const IGNORED_PROCESSES: &[&str] = &[
    "System",
    "System Idle Process",
    "Idle",
    "Registry",
    "Memory Compression",
    "MemCompression",
    "dwm.exe",
    "explorer.exe",
    "csrss.exe",
    "speedimprover.exe",
];

/// Runs every rule; findings are sorted High → Low, then by id.
pub fn analyze(s: &SystemSnapshot, games: &[InstalledGame]) -> Vec<Finding> {
    let mut findings: Vec<Finding> = [
        refresh_below_max(s),
        on_battery(s),
        plan_saver(s),
        single_channel(s),
        below_rated(s),
        jedec_default(s),
        memory_pressure(s),
        low_total(s),
        game_mode_off(s),
        background_recording_on(s),
        hags_off(s),
        memory_integrity_on(s),
        driver_age(s),
        low_free(s),
        games_on_hdd(s, games),
        process_hogs(s),
        too_many_startup_apps(s),
    ]
    .into_iter()
    .flatten()
    .collect();
    findings.sort_by(|a, b| b.severity.cmp(&a.severity).then(a.id.cmp(b.id)));
    findings
}

fn finding(
    id: &'static str,
    severity: Severity,
    category: Category,
    title: &str,
    detail: String,
    fix: Fix,
) -> Option<Finding> {
    Some(Finding {
        id,
        severity,
        category,
        title: title.to_string(),
        detail,
        fix,
    })
}

fn uri(u: &str) -> Fix {
    Fix::OpenUri(u.to_string())
}

fn manual(text: &str) -> Fix {
    Fix::Manual(text.to_string())
}

fn gb(mb: u64) -> String {
    format!("{:.1} GB", mb as f64 / 1024.0)
}

fn is_ignored(name: &str) -> bool {
    IGNORED_PROCESSES
        .iter()
        .any(|i| i.eq_ignore_ascii_case(name))
}

fn refresh_below_max(s: &SystemSnapshot) -> Option<Finding> {
    let slow: Vec<String> = s
        .displays
        .iter()
        .filter(|d| d.current_hz > 0 && d.max_hz >= d.current_hz + 3)
        .map(|d| {
            format!(
                "{} runs at {} Hz but supports {} Hz",
                d.name, d.current_hz, d.max_hz
            )
        })
        .collect();
    if slow.is_empty() {
        return None;
    }
    finding(
        "display.refresh_below_max",
        Severity::High,
        Category::Display,
        "Monitor running below its max refresh rate",
        format!(
            "{}. Games can never show more frames than the monitor refreshes.",
            slow.join("; ")
        ),
        uri("ms-settings:display-advanced"),
    )
}

fn on_battery(s: &SystemSnapshot) -> Option<Finding> {
    if s.machine.is_laptop != Some(true) || s.machine.on_battery != Some(true) {
        return None;
    }
    finding(
        "power.on_battery",
        Severity::High,
        Category::Power,
        "Laptop is running on battery",
        "On battery, Windows and the GPU driver cut CPU and GPU power to save energy.".into(),
        manual("Plug in your charger"),
    )
}

fn plan_saver(s: &SystemSnapshot) -> Option<Finding> {
    if s.os.power_plan != PowerPlan::PowerSaver {
        return None;
    }
    finding(
        "power.plan_saver",
        Severity::High,
        Category::Power,
        "Power saver plan is active",
        "The Power saver plan caps CPU speed. Switch to Balanced or High performance.".into(),
        uri("ms-settings:powersleep"),
    )
}

fn single_channel(s: &SystemSnapshot) -> Option<Finding> {
    if s.memory.modules.len() != 1 {
        return None;
    }
    let severity = if s.has_discrete_gpu() {
        Severity::Medium
    } else {
        Severity::High
    };
    finding(
        "memory.single_channel",
        severity,
        Category::Memory,
        "RAM is running in single-channel mode",
        "Only one RAM stick is installed, which halves memory bandwidth. Integrated graphics suffer most.".into(),
        manual("Install a second matching RAM stick"),
    )
}

fn below_rated(s: &SystemSnapshot) -> Option<Finding> {
    let m = s
        .memory
        .modules
        .iter()
        .find(|m| m.configured_mhz > 0 && m.configured_mhz < m.rated_mhz)?;
    finding(
        "memory.below_rated",
        Severity::High,
        Category::Memory,
        "RAM is running below its rated speed",
        format!(
            "RAM at {} MT/s, rated {} MT/s.",
            m.configured_mhz, m.rated_mhz
        ),
        manual("Enable XMP/EXPO in BIOS"),
    )
}

fn jedec_default(s: &SystemSnapshot) -> Option<Finding> {
    if s.machine.is_laptop != Some(false) || below_rated(s).is_some() {
        return None;
    }
    let m = s.memory.modules.iter().find(|m| {
        m.configured_mhz > 0
            && match m.generation {
                MemoryGen::Ddr4 => m.configured_mhz <= 2666,
                MemoryGen::Ddr5 => m.configured_mhz <= 4800,
                MemoryGen::Other => false,
            }
    })?;
    finding(
        "memory.jedec_default",
        Severity::Low,
        Category::Memory,
        "RAM may be running at default speed",
        format!(
            "RAM runs at {} MT/s, the standard default for its type. Most gaming kits are rated faster.",
            m.configured_mhz
        ),
        manual("Enable XMP/EXPO in BIOS if your kit is rated higher"),
    )
}

fn memory_pressure(s: &SystemSnapshot) -> Option<Finding> {
    let mem = &s.memory;
    if mem.total_mb == 0 {
        return None;
    }
    let ratio = mem.used_mb as f64 / mem.total_mb as f64;
    let severity = if ratio >= 0.85 {
        Severity::High
    } else if ratio >= 0.75 {
        Severity::Medium
    } else {
        return None;
    };
    let mut biggest: Vec<_> = s
        .processes
        .iter()
        .filter(|p| !is_ignored(&p.name))
        .collect();
    biggest.sort_by_key(|p| std::cmp::Reverse(p.mem_mb));
    let names: Vec<&str> = biggest.iter().take(3).map(|p| p.name.as_str()).collect();
    let mut detail = format!(
        "{} of {} in use ({:.0} %).",
        gb(mem.used_mb),
        gb(mem.total_mb),
        ratio * 100.0
    );
    if !names.is_empty() {
        detail.push_str(&format!(" Biggest apps: {}.", names.join(", ")));
    }
    finding(
        "memory.pressure",
        severity,
        Category::Memory,
        "RAM is almost full",
        detail,
        manual("Close the apps listed above before playing"),
    )
}

fn low_total(s: &SystemSnapshot) -> Option<Finding> {
    let total = s.memory.total_mb;
    let severity = match total {
        0 => return None,
        t if t < 7_500 => Severity::High,
        t if t < 15_000 => Severity::Medium,
        _ => return None,
    };
    finding(
        "memory.low_total",
        severity,
        Category::Memory,
        "Not enough RAM for modern games",
        format!("{} installed; most current games want 16 GB.", gb(total)),
        manual("Upgrade to 16 GB of RAM"),
    )
}

fn game_mode_off(s: &SystemSnapshot) -> Option<Finding> {
    if s.os.game_mode != Some(false) {
        return None;
    }
    finding(
        "os.game_mode_off",
        Severity::Medium,
        Category::Os,
        "Game Mode is off",
        "Game Mode stops Windows Update and background tasks from interrupting games.".into(),
        uri("ms-settings:gaming-gamemode"),
    )
}

fn background_recording_on(s: &SystemSnapshot) -> Option<Finding> {
    if s.os.background_recording != Some(true) {
        return None;
    }
    finding(
        "os.background_recording_on",
        Severity::Medium,
        Category::Os,
        "Background recording is on",
        "Xbox Game Bar is constantly recording the last minutes of gameplay, which costs GPU time."
            .into(),
        uri("ms-settings:gaming-gamedvr"),
    )
}

fn hags_off(s: &SystemSnapshot) -> Option<Finding> {
    if s.os.hags != Some(false) {
        return None;
    }
    finding(
        "os.hags_off",
        Severity::Low,
        Category::Os,
        "Hardware-accelerated GPU scheduling is off",
        "Letting the GPU manage its own memory can reduce latency and is required for DLSS Frame Generation.".into(),
        uri("ms-settings:display-advancedgraphics"),
    )
}

fn memory_integrity_on(s: &SystemSnapshot) -> Option<Finding> {
    if s.os.memory_integrity != Some(true) {
        return None;
    }
    finding(
        "os.memory_integrity_on",
        Severity::Low,
        Category::Os,
        "Memory Integrity is on",
        "Memory Integrity (Core isolation) can cost a few percent of FPS. It is a security feature: \
         turning it off makes the PC easier to attack, so only do it if you accept that trade-off."
            .into(),
        uri("windowsdefender://coreisolation/"),
    )
}

fn driver_age(s: &SystemSnapshot) -> Option<Finding> {
    let gpu = s.primary_gpu()?;
    let date = gpu.driver_date.as_deref()?;
    let days = s.age_days(date)?;
    let severity = if days > 365 {
        Severity::High
    } else if days > 180 {
        Severity::Medium
    } else {
        return None;
    };
    finding(
        "gpu.driver_age",
        severity,
        Category::Gpu,
        "Graphics driver is out of date",
        format!(
            "The {} driver is from {date} ({days} days old). New drivers often add game-specific fixes.",
            gpu.name
        ),
        Fix::ShowDrivers,
    )
}

fn low_free(s: &SystemSnapshot) -> Option<Finding> {
    let d = s.system_disk()?;
    if d.total_gb <= 0.0 {
        return None;
    }
    if d.free_gb / d.total_gb >= 0.10 && d.free_gb >= 20.0 {
        return None;
    }
    finding(
        "storage.low_free",
        Severity::Medium,
        Category::Storage,
        "System drive is almost full",
        format!(
            "Only {:.1} GB free of {:.0} GB. Windows needs free space for the page file and updates.",
            d.free_gb, d.total_gb
        ),
        uri("ms-settings:storagesense"),
    )
}

/// Lower-case with `\` separators, so `D:/Games` and `d:\games` compare equal.
fn normalize_path(p: &str) -> String {
    p.to_ascii_lowercase().replace('/', "\\")
}

/// The disk whose mount point is the longest prefix of `path` (case and
/// separator insensitive).
fn disk_for<'a>(disks: &'a [DiskInfo], path: &str) -> Option<&'a DiskInfo> {
    let path = normalize_path(path);
    disks
        .iter()
        .filter(|d| !d.mount.is_empty() && path.starts_with(&normalize_path(&d.mount)))
        .max_by_key(|d| d.mount.len())
}

fn games_on_hdd(s: &SystemSnapshot, games: &[InstalledGame]) -> Option<Finding> {
    let on_hdd: Vec<&str> = games
        .iter()
        .filter(|g| disk_for(&s.disks, &g.install_dir).is_some_and(|d| d.media == DiskMedia::Hdd))
        .map(|g| g.name.as_str())
        .collect();
    if on_hdd.is_empty() {
        return None;
    }
    finding(
        "storage.games_on_hdd",
        Severity::Medium,
        Category::Storage,
        "Games installed on a hard drive",
        format!(
            "{} {} on a hard drive, which causes long loads and stutter when new areas stream in.",
            on_hdd.join(", "),
            if on_hdd.len() == 1 { "is" } else { "are" }
        ),
        manual("Move these games to an SSD"),
    )
}

fn process_hogs(s: &SystemSnapshot) -> Option<Finding> {
    let mut hogs: Vec<_> = s
        .processes
        .iter()
        .filter(|p| !is_ignored(&p.name) && (p.cpu_pct >= 10.0 || p.mem_mb >= 1_500))
        .collect();
    if hogs.is_empty() {
        return None;
    }
    hogs.sort_by(|a, b| b.cpu_pct.total_cmp(&a.cpu_pct));
    let top: Vec<String> = hogs
        .iter()
        .take(3)
        .map(|p| format!("{} ({:.0} % CPU, {})", p.name, p.cpu_pct, gb(p.mem_mb)))
        .collect();
    finding(
        "process.hogs",
        Severity::Medium,
        Category::Processes,
        "Background apps are using resources",
        format!("{}.", top.join(", ")),
        manual("Close these apps before playing"),
    )
}

fn too_many_startup_apps(s: &SystemSnapshot) -> Option<Finding> {
    let n = s.os.startup_apps?;
    if n <= 15 {
        return None;
    }
    finding(
        "startup.too_many",
        Severity::Low,
        Category::Startup,
        "Too many startup apps",
        format!("{n} apps start with Windows and keep running in the background."),
        uri("ms-settings:startupapps"),
    )
}
