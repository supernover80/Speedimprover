//! Choosing CPU and GPU temperatures from the sources Windows offers.
//!
//! Exact CPU temperatures need a kernel driver, so we read them from
//! LibreHardwareMonitor (or OpenHardwareMonitor) when the user runs it; it
//! publishes its sensors over WMI. Otherwise the ACPI thermal zones give an
//! approximate reading. These helpers are pure so they test on every OS.

use serde::Deserialize;

const TEMPERATURE: &str = "Temperature";
const CPU_PREFIXES: [&str; 2] = ["/intelcpu", "/amdcpu"];
const CPU_PREFERRED: [&str; 4] = ["CPU Package", "Core (Tctl/Tdie)", "Tctl/Tdie", "Package"];
const GPU_PREFIXES: [&str; 3] = ["/gpu-nvidia", "/gpu-amd", "/gpu-intel"];
const GPU_PREFERRED: [&str; 1] = ["GPU Core"];
/// Readings outside this range are sensor glitches.
const PLAUSIBLE_C: std::ops::Range<f32> = 0.5..125.0;
const KELVIN_OFFSET: f64 = 273.15;

/// One row of LibreHardwareMonitor's / OpenHardwareMonitor's WMI `Sensor` class.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(rename = "Sensor", rename_all = "PascalCase")]
pub struct LhmSensor {
    /// e.g. `/intelcpu/0/temperature/8`.
    pub identifier: Option<String>,
    /// e.g. `CPU Package`.
    pub name: Option<String>,
    /// e.g. `Temperature`, `Load`.
    pub sensor_type: Option<String>,
    pub value: Option<f32>,
}

fn pick(rows: &[LhmSensor], prefixes: &[&str], preferred: &[&str]) -> Option<f32> {
    let temps: Vec<(&str, f32)> = rows
        .iter()
        .filter(|s| {
            s.sensor_type
                .as_deref()
                .is_some_and(|t| t.eq_ignore_ascii_case(TEMPERATURE))
                && s.identifier.as_deref().is_some_and(|id| {
                    let id = id.to_ascii_lowercase();
                    prefixes.iter().any(|p| id.starts_with(p))
                })
        })
        .filter_map(|s| {
            let value = s.value.filter(|v| PLAUSIBLE_C.contains(v))?;
            Some((s.name.as_deref().unwrap_or(""), value))
        })
        .collect();
    preferred
        .iter()
        .find_map(|want| {
            temps
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(want))
                .map(|&(_, v)| v)
        })
        .or_else(|| temps.iter().map(|&(_, v)| v).reduce(f32::max))
}

/// CPU package temperature (°C) from LibreHardwareMonitor sensors.
pub fn cpu_temp_from_sensors(rows: &[LhmSensor]) -> Option<f32> {
    pick(rows, &CPU_PREFIXES, &CPU_PREFERRED)
}

/// GPU core temperature (°C) from LibreHardwareMonitor sensors.
pub fn gpu_temp_from_sensors(rows: &[LhmSensor]) -> Option<f32> {
    pick(rows, &GPU_PREFIXES, &GPU_PREFERRED)
}

/// Hottest ACPI thermal zone in °C (rounded to 0.1) from PDH readings in Kelvin.
pub fn thermal_zone_celsius(readings: &[(String, f64)]) -> Option<f32> {
    readings
        .iter()
        .map(|&(_, kelvin)| (kelvin - KELVIN_OFFSET) as f32)
        .filter(|c| c.is_finite() && PLAUSIBLE_C.contains(c))
        .reduce(f32::max)
        .map(|c| (c * 10.0).round() / 10.0)
}

/// `D3DKMT_ADAPTER_PERFDATA.Temperature` (tenths of °C; 0 = not supported).
pub fn deci_celsius(raw: u32) -> Option<f32> {
    Some(raw as f32 / 10.0).filter(|c| PLAUSIBLE_C.contains(c))
}
