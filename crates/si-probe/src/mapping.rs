//! WMI row types and the pure functions that turn raw Windows data into the
//! snapshot model. Nothing here calls Windows, so it is tested on every OS.

use std::collections::HashMap;

use chrono::{Days, NaiveDate};
use serde::Deserialize;
use si_core::model::{DiskMedia, DriverInfo, GpuInfo, MemoryGen, MemoryModule, Vendor};

const MB: u64 = 1024 * 1024;
const DXGI_ADAPTER_FLAG_SOFTWARE: u32 = 2;
const PCI_VENDOR_MICROSOFT: u32 = 0x1414;
const SMBIOS_DDR4: u32 = 26;
const SMBIOS_DDR5: u32 = 34;
const STORAGE_MEDIA_HDD: u16 = 3;
const STORAGE_MEDIA_SSD: u16 = 4;
const STORAGE_BUS_NVME: u16 = 17;
/// `Win32_DeviceGuard.SecurityServicesRunning` value for Memory Integrity (HVCI).
const DEVICE_GUARD_HVCI: u32 = 2;

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(rename = "Win32_Processor", rename_all = "PascalCase")]
pub struct Win32Processor {
    pub name: Option<String>,
    pub number_of_cores: Option<u32>,
    pub number_of_logical_processors: Option<u32>,
    pub max_clock_speed: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(rename = "Win32_PhysicalMemory", rename_all = "PascalCase")]
pub struct Win32PhysicalMemory {
    /// uint64 properties arrive from WMI as strings.
    pub capacity: Option<String>,
    pub speed: Option<u32>,
    pub configured_clock_speed: Option<u32>,
    #[serde(rename = "SMBIOSMemoryType")]
    pub smbios_memory_type: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(rename = "Win32_VideoController", rename_all = "PascalCase")]
pub struct Win32VideoController {
    pub name: Option<String>,
    pub driver_version: Option<String>,
    pub driver_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(rename = "Win32_PnPSignedDriver", rename_all = "PascalCase")]
pub struct Win32PnPSignedDriver {
    pub device_name: Option<String>,
    pub device_class: Option<String>,
    pub manufacturer: Option<String>,
    pub driver_version: Option<String>,
    pub driver_date: Option<String>,
    #[serde(rename = "HardWareID")]
    pub hardware_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(rename = "MSFT_PhysicalDisk", rename_all = "PascalCase")]
pub struct MsftPhysicalDisk {
    pub device_id: Option<String>,
    pub media_type: Option<u16>,
    pub bus_type: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(rename = "MSFT_Partition", rename_all = "PascalCase")]
pub struct MsftPartition {
    pub disk_number: Option<u32>,
    /// UTF-16 code unit; 0 when the partition has no letter.
    pub drive_letter: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(rename = "Win32_DeviceGuard", rename_all = "PascalCase")]
pub struct Win32DeviceGuard {
    pub security_services_running: Option<Vec<u32>>,
}

/// CIM datetime (`20240115000000.000000-000`, also `20240910000000.******+***`)
/// → `2024-01-15`.
pub fn parse_wmi_date(s: &str) -> Option<String> {
    let ymd = s.get(..8)?;
    let date = NaiveDate::parse_from_str(ymd, "%Y%m%d").ok()?;
    Some(date.format("%Y-%m-%d").to_string())
}

/// OLE Automation date (days since 1899-12-30) → `YYYY-MM-DD`.
pub fn ole_date_to_ymd(days: f64) -> Option<String> {
    if !days.is_finite() || days < 0.0 {
        return None;
    }
    let epoch = NaiveDate::from_ymd_opt(1899, 12, 30)?;
    let date = epoch.checked_add_days(Days::new(days.floor() as u64))?;
    Some(date.format("%Y-%m-%d").to_string())
}

pub fn memory_module(row: &Win32PhysicalMemory) -> MemoryModule {
    MemoryModule {
        capacity_mb: row
            .capacity
            .as_deref()
            .and_then(|c| c.trim().parse::<u64>().ok())
            .map_or(0, |bytes| bytes / MB),
        rated_mhz: row.speed.unwrap_or(0),
        configured_mhz: row.configured_clock_speed.unwrap_or(0),
        generation: match row.smbios_memory_type {
            Some(SMBIOS_DDR4) => MemoryGen::Ddr4,
            Some(SMBIOS_DDR5) => MemoryGen::Ddr5,
            _ => MemoryGen::Other,
        },
    }
}

pub fn vendor_from_pci_id(id: u32) -> Vendor {
    match id {
        0x10DE => Vendor::Nvidia,
        0x1002 | 0x1022 => Vendor::Amd,
        0x8086 => Vendor::Intel,
        _ => Vendor::Other,
    }
}

/// Software rasterizers and Microsoft's basic display/render adapters.
pub fn is_skipped_adapter(vendor_id: u32, flags: u32) -> bool {
    vendor_id == PCI_VENDOR_MICROSOFT || flags & DXGI_ADAPTER_FLAG_SOFTWARE != 0
}

/// Fallback when D3D12 cannot tell whether a GPU shares system memory.
pub fn integrated_by_name(name: &str) -> bool {
    let n = name.to_lowercase().replace("(r)", "").replace("(tm)", "");
    let tokens: Vec<&str> = n.split_whitespace().collect();
    let mobile_radeon = tokens.windows(2).any(|w| {
        w[0] == "radeon"
            && w[1]
                .strip_suffix('m')
                .is_some_and(|d| d.len() == 3 && d.bytes().all(|b| b.is_ascii_digit()))
    });
    n.contains("uhd graphics")
        || n.contains("hd graphics")
        || n.contains("iris")
        || n.contains("radeon graphics")
        || n.contains("arc graphics")
        || n.contains("vega")
        || mobile_radeon
}

pub fn disk_media(media_type: u16, bus_type: u16) -> DiskMedia {
    if bus_type == STORAGE_BUS_NVME {
        DiskMedia::Nvme
    } else if media_type == STORAGE_MEDIA_SSD {
        DiskMedia::Ssd
    } else if media_type == STORAGE_MEDIA_HDD {
        DiskMedia::Hdd
    } else {
        DiskMedia::Unknown
    }
}

/// Drive letter (upper case) → media of the physical disk it lives on.
pub fn media_by_letter(
    disks: &[MsftPhysicalDisk],
    parts: &[MsftPartition],
) -> HashMap<char, DiskMedia> {
    let by_number: HashMap<u32, DiskMedia> = disks
        .iter()
        .filter_map(|d| {
            let number = d.device_id.as_deref()?.trim().parse().ok()?;
            Some((
                number,
                disk_media(d.media_type.unwrap_or(0), d.bus_type.unwrap_or(0)),
            ))
        })
        .collect();
    parts
        .iter()
        .filter_map(|p| {
            let letter = char::from_u32(u32::from(p.drive_letter?))
                .filter(|c| c.is_ascii_alphabetic())?
                .to_ascii_uppercase();
            Some((letter, *by_number.get(&p.disk_number?)?))
        })
        .collect()
}

/// WMI's upper-case device class → the spelling used across the app.
fn normalize_class(class: &str) -> String {
    match class.to_ascii_uppercase().as_str() {
        "DISPLAY" => "Display".into(),
        "NET" => "Net".into(),
        "MEDIA" => "Media".into(),
        "SCSIADAPTER" => "SCSIAdapter".into(),
        "HDC" => "HDC".into(),
        "BLUETOOTH" => "Bluetooth".into(),
        "SYSTEM" => "System".into(),
        other => {
            let lower = other.to_ascii_lowercase();
            let mut chars = lower.chars();
            chars
                .next()
                .map(|c| c.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        }
    }
}

fn non_empty(s: &Option<String>) -> Option<&str> {
    s.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

/// `None` when the row lacks a device name or driver version.
pub fn driver_info(row: &Win32PnPSignedDriver) -> Option<DriverInfo> {
    Some(DriverInfo {
        device_name: non_empty(&row.device_name)?.to_string(),
        class: non_empty(&row.device_class)
            .map(normalize_class)
            .unwrap_or_default(),
        manufacturer: non_empty(&row.manufacturer).unwrap_or_default().to_string(),
        version: non_empty(&row.driver_version)?.to_string(),
        date: row.driver_date.as_deref().and_then(parse_wmi_date),
        hardware_ids: non_empty(&row.hardware_id)
            .map(|id| vec![id.to_string()])
            .unwrap_or_default(),
    })
}

/// Copies driver version/date from `Win32_VideoController` onto the DXGI GPUs,
/// matching by name (trimmed, case-insensitive).
pub fn attach_gpu_drivers(gpus: &mut [GpuInfo], controllers: &[Win32VideoController]) {
    for gpu in gpus.iter_mut() {
        let found = controllers
            .iter()
            .find(|c| non_empty(&c.name).is_some_and(|n| n.eq_ignore_ascii_case(gpu.name.trim())));
        if let Some(c) = found {
            gpu.driver_version = non_empty(&c.driver_version).map(str::to_string);
            gpu.driver_date = c.driver_date.as_deref().and_then(parse_wmi_date);
        }
    }
}

pub fn memory_integrity_on(row: &Win32DeviceGuard) -> Option<bool> {
    row.security_services_running
        .as_ref()
        .map(|running| running.contains(&DEVICE_GUARD_HVCI))
}
