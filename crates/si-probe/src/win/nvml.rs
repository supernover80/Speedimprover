//! NVIDIA GPU telemetry via NVML (`nvml.dll`, installed with the driver).
//! On other GPUs, or without the DLL, `NvmlReader::new` returns `None`.

use nvml_wrapper::Nvml;
use nvml_wrapper::bitmasks::device::ThrottleReasons;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;

const MB: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NvidiaReading {
    pub temp_c: f32,
    pub util_pct: f32,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    /// Thermal slowdown only: the power cap is normal under full load.
    pub throttling: bool,
}

pub struct NvmlReader {
    nvml: Nvml,
}

impl NvmlReader {
    pub fn new() -> Option<Self> {
        let nvml = Nvml::init().ok()?;
        (nvml.device_count().ok()? > 0).then_some(Self { nvml })
    }

    /// Reads the first NVIDIA GPU.
    pub fn read(&self) -> Option<NvidiaReading> {
        let device = self.nvml.device_by_index(0).ok()?;
        let memory = device.memory_info().ok()?;
        let thermal = ThrottleReasons::HW_SLOWDOWN
            | ThrottleReasons::HW_THERMAL_SLOWDOWN
            | ThrottleReasons::SW_THERMAL_SLOWDOWN;
        Some(NvidiaReading {
            temp_c: device.temperature(TemperatureSensor::Gpu).ok()? as f32,
            util_pct: device.utilization_rates().ok()?.gpu as f32,
            mem_used_mb: memory.used / MB,
            mem_total_mb: memory.total / MB,
            throttling: device
                .current_throttle_reasons()
                .is_ok_and(|r| r.intersects(thermal)),
        })
    }
}
