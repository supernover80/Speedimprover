//! Which drivers need updating (spec §7).
//!
//! Evidence, strongest first: Windows Update offers a driver for the device;
//! the version catalog lists a newer version; the driver is over a year old.
//! Without evidence the status is "no update found", never "up to date".

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::model::{DriverInfo, SystemSnapshot};

const BUNDLED_CATALOG: &str = include_str!("../../../data/driver_catalog.json");
const OPTIONAL_UPDATES_URI: &str = "ms-settings:windowsupdate-optionalupdates";
/// Device classes shown on the Drivers page (plus chipset `System` drivers).
const SHOWN_CLASSES: [&str; 6] = ["Display", "Net", "Media", "SCSIAdapter", "HDC", "Bluetooth"];
const POSSIBLY_OUTDATED_DAYS: i64 = 365;

/// Compares dotted numeric versions; missing parts count as 0.
/// `None` if either side is empty or has a non-numeric part.
pub fn compare_versions(a: &str, b: &str) -> Option<Ordering> {
    let (a, b) = (numeric_parts(a)?, numeric_parts(b)?);
    let len = a.len().max(b.len());
    let part = |v: &[u64], i: usize| v.get(i).copied().unwrap_or(0);
    Some(
        (0..len)
            .map(|i| part(&a, i).cmp(&part(&b, i)))
            .find(|o| o.is_ne())
            .unwrap_or(Ordering::Equal),
    )
}

fn numeric_parts(v: &str) -> Option<Vec<u64>> {
    let v = v.trim();
    if v.is_empty() {
        return None;
    }
    v.split('.').map(|p| p.parse().ok()).collect()
}

/// Windows driver version → NVIDIA's version: `32.0.15.6094` → `560.94`
/// (digits of the last two fields, last five digits, dot before the final two).
pub fn nvidia_display_version(windows_version: &str) -> Option<String> {
    let fields: Vec<&str> = windows_version.trim().split('.').collect();
    if fields.len() != 4 {
        return None;
    }
    let digits = format!("{}{}", fields[2], fields[3]);
    if digits.len() < 5 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let last5 = &digits[digits.len() - 5..];
    Some(format!("{}.{}", &last5[..3], &last5[3..]))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DriverVendor {
    Nvidia,
    Amd,
    Intel,
}

/// Vendor from a driver's `Manufacturer` string.
pub fn driver_vendor(manufacturer: &str) -> Option<DriverVendor> {
    let m = manufacturer.to_ascii_lowercase();
    let has_word = |w: &str| {
        m.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|x| x == w)
    };
    if m.contains("nvidia") {
        Some(DriverVendor::Nvidia)
    } else if m.contains("advanced micro devices") || has_word("amd") {
        Some(DriverVendor::Amd)
    } else if has_word("intel") {
        Some(DriverVendor::Intel)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub vendor: DriverVendor,
    pub class: String,
    /// NVIDIA format (`621.24`) for NVIDIA, Windows format for AMD and Intel.
    pub latest_version: String,
    pub download_url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverCatalog {
    pub schema: u32,
    pub updated: String,
    pub entries: Vec<CatalogEntry>,
}

impl DriverCatalog {
    pub fn parse(json: &str) -> Result<Self, CoreError> {
        Ok(serde_json::from_str(json)?)
    }

    /// The catalog compiled into the binary (used when the online copy is unreachable).
    pub fn bundled() -> Self {
        Self::parse(BUNDLED_CATALOG).unwrap_or_else(|_| Self::empty())
    }

    pub fn empty() -> Self {
        Self {
            schema: 1,
            updated: String::new(),
            entries: Vec::new(),
        }
    }

    fn entry_for(&self, vendor: DriverVendor, class: &str) -> Option<&CatalogEntry> {
        self.entries
            .iter()
            .find(|e| e.vendor == vendor && e.class.eq_ignore_ascii_case(class))
    }
}

/// A driver Windows Update offers but has not installed.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AvailableDriverUpdate {
    pub title: String,
    pub hardware_id: String,
    pub driver_class: String,
    pub driver_model: String,
    pub ver_date: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateSource {
    WindowsUpdate,
    Catalog,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriverStatus {
    UpdateAvailable {
        source: UpdateSource,
        latest: Option<String>,
        /// Where the "Update" button goes.
        url: String,
    },
    PossiblyOutdated {
        age_days: i64,
    },
    NoUpdateFound,
}

impl DriverStatus {
    fn rank(&self) -> u8 {
        match self {
            DriverStatus::UpdateAvailable { .. } => 0,
            DriverStatus::PossiblyOutdated { .. } => 1,
            DriverStatus::NoUpdateFound => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverRow {
    pub driver: DriverInfo,
    pub status: DriverStatus,
}

/// Drivers that matter for gaming: graphics, network, audio, storage,
/// Bluetooth, and Intel/AMD chipset (`System`) drivers.
pub fn is_shown_class(d: &DriverInfo) -> bool {
    SHOWN_CLASSES
        .iter()
        .any(|c| c.eq_ignore_ascii_case(&d.class))
        || (d.class.eq_ignore_ascii_case("System")
            && matches!(
                driver_vendor(&d.manufacturer),
                Some(DriverVendor::Intel | DriverVendor::Amd)
            ))
}

/// Status of every shown driver; updates first, then possibly outdated, then
/// the rest, each group sorted by device name.
pub fn evaluate_drivers(
    s: &SystemSnapshot,
    catalog: &DriverCatalog,
    wu: &[AvailableDriverUpdate],
) -> Vec<DriverRow> {
    let mut rows: Vec<DriverRow> = s
        .drivers
        .iter()
        .filter(|d| is_shown_class(d))
        .map(|d| DriverRow {
            driver: d.clone(),
            status: status_for(s, d, catalog, wu),
        })
        .collect();
    rows.sort_by(|a, b| {
        a.status
            .rank()
            .cmp(&b.status.rank())
            .then_with(|| a.driver.device_name.cmp(&b.driver.device_name))
    });
    rows
}

fn status_for(
    s: &SystemSnapshot,
    d: &DriverInfo,
    catalog: &DriverCatalog,
    wu: &[AvailableDriverUpdate],
) -> DriverStatus {
    let offered = wu.iter().any(|u| {
        d.hardware_ids
            .iter()
            .any(|h| h.eq_ignore_ascii_case(&u.hardware_id))
    });
    if offered {
        return DriverStatus::UpdateAvailable {
            source: UpdateSource::WindowsUpdate,
            latest: None,
            url: OPTIONAL_UPDATES_URI.to_string(),
        };
    }
    if let Some(status) = catalog_status(d, catalog) {
        return status;
    }
    match d.date.as_deref().and_then(|date| s.age_days(date)) {
        Some(age_days) if age_days > POSSIBLY_OUTDATED_DAYS => {
            DriverStatus::PossiblyOutdated { age_days }
        }
        _ => DriverStatus::NoUpdateFound,
    }
}

fn catalog_status(d: &DriverInfo, catalog: &DriverCatalog) -> Option<DriverStatus> {
    let vendor = driver_vendor(&d.manufacturer)?;
    let entry = catalog.entry_for(vendor, &d.class)?;
    let installed = if vendor == DriverVendor::Nvidia && d.class.eq_ignore_ascii_case("Display") {
        nvidia_display_version(&d.version)?
    } else {
        d.version.clone()
    };
    (compare_versions(&installed, &entry.latest_version)? == Ordering::Less).then(|| {
        DriverStatus::UpdateAvailable {
            source: UpdateSource::Catalog,
            latest: Some(entry.latest_version.clone()),
            url: entry.download_url.clone(),
        }
    })
}
