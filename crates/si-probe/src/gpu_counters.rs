//! Interpreting the PDH `\GPU Engine(*)\Utilization Percentage` counter.
//!
//! Each instance is one engine of one adapter used by one process, named like
//! `pid_1234_luid_0x00000000_0x0000D1A5_phys_0_eng_0_engtype_3D`.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineInstance {
    pub pid: u32,
    /// Adapter id, e.g. `0x00000000_0x0000D1A5`.
    pub luid: String,
    pub is_3d: bool,
}

pub fn parse_gpu_engine_instance(name: &str) -> Option<EngineInstance> {
    let rest = name.strip_prefix("pid_")?;
    let (pid, rest) = rest.split_once('_')?;
    let rest = rest.strip_prefix("luid_")?;
    let (luid, rest) = rest.split_once("_phys_")?;
    Some(EngineInstance {
        pid: pid.parse().ok()?,
        luid: luid.to_string(),
        is_3d: rest.to_ascii_lowercase().ends_with("engtype_3d"),
    })
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GpuAggregate {
    /// Busiest adapter's 3D load, 0–100.
    pub total_pct: f32,
    /// 3D load per process id, 0–100.
    pub per_pid: HashMap<u32, f32>,
}

/// Sums 3D-engine load per adapter and per process; other engines (copy,
/// video decode) are ignored. The total is the busiest adapter, clamped to 100.
pub fn aggregate_gpu(readings: &[(String, f64)]) -> GpuAggregate {
    let mut by_adapter: HashMap<String, f64> = HashMap::new();
    let mut per_pid: HashMap<u32, f64> = HashMap::new();
    for (name, value) in readings {
        let Some(engine) = parse_gpu_engine_instance(name) else {
            continue;
        };
        if !engine.is_3d || !value.is_finite() {
            continue;
        }
        *by_adapter.entry(engine.luid).or_default() += value;
        *per_pid.entry(engine.pid).or_default() += value;
    }
    let clamp = |v: f64| v.clamp(0.0, 100.0) as f32;
    GpuAggregate {
        total_pct: clamp(by_adapter.values().copied().fold(0.0, f64::max)),
        per_pid: per_pid
            .into_iter()
            .map(|(pid, v)| (pid, clamp(v)))
            .collect(),
    }
}
