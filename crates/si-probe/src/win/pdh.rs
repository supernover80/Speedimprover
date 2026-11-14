//! Performance counters (PDH). Counters are added by their English names with
//! `PdhAddEnglishCounterW`, so this works on non-English Windows too.

use std::ptr::null_mut;

use windows::Win32::System::Performance::{
    PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
    PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW, PdhCloseQuery,
    PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};
use windows::core::{HRESULT, PCWSTR, w};

const MB: f64 = 1024.0 * 1024.0;

/// One reading of the PDH counters this app uses.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PdhReading {
    /// `(instance name, % utilization)` for every 3D engine instance.
    pub gpu_instances: Vec<(String, f64)>,
    /// Dedicated VRAM in use on the busiest adapter.
    pub vram_used_mb: Option<u64>,
    /// `% Processor Performance` (below 100 = running under base clock).
    pub cpu_perf_pct: Option<f32>,
}

pub struct PdhSampler {
    query: PDH_HQUERY,
    gpu_engine: Option<PDH_HCOUNTER>,
    gpu_memory: Option<PDH_HCOUNTER>,
    cpu_perf: Option<PDH_HCOUNTER>,
}

fn check(status: u32) -> windows::core::Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(HRESULT(status as i32).into())
    }
}

impl PdhSampler {
    /// Opens the query. A counter that does not exist on this PC (e.g. GPU
    /// counters in a VM) is skipped; only failing to open the query is an error.
    pub fn new() -> windows::core::Result<Self> {
        let mut query = PDH_HQUERY(null_mut());
        // SAFETY: valid out-pointer; a null data source means live data.
        check(unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) })?;
        let add = |path: PCWSTR| {
            let mut counter = PDH_HCOUNTER(null_mut());
            // SAFETY: `query` is open; `path` is a static wide string.
            check(unsafe { PdhAddEnglishCounterW(query, path, 0, &mut counter) })
                .ok()
                .map(|_| counter)
        };
        let sampler = Self {
            query,
            gpu_engine: add(w!(r"\GPU Engine(*engtype_3D)\Utilization Percentage")),
            gpu_memory: add(w!(r"\GPU Adapter Memory(*)\Dedicated Usage")),
            cpu_perf: add(w!(
                r"\Processor Information(_Total)\% Processor Performance"
            )),
        };
        // Rate counters need a first collection to compare against.
        // SAFETY: `query` is open.
        unsafe { PdhCollectQueryData(query) };
        Ok(sampler)
    }

    pub fn collect(&mut self) -> PdhReading {
        // SAFETY: `self.query` stays open until Drop.
        if check(unsafe { PdhCollectQueryData(self.query) }).is_err() {
            return PdhReading::default();
        }
        let gpu_instances = self.gpu_engine.map(read_array).unwrap_or_default();
        let vram_used_mb = self
            .gpu_memory
            .map(read_array)
            .filter(|v| !v.is_empty())
            .map(|v| (v.iter().map(|(_, bytes)| *bytes).fold(0.0, f64::max) / MB) as u64);
        let cpu_perf_pct = self
            .cpu_perf
            .map(read_array)
            .and_then(|v| v.first().map(|(_, pct)| *pct as f32));
        PdhReading {
            gpu_instances,
            vram_used_mb,
            cpu_perf_pct,
        }
    }
}

impl Drop for PdhSampler {
    fn drop(&mut self) {
        // SAFETY: the query was opened in `new` and is closed exactly once.
        unsafe { PdhCloseQuery(self.query) };
    }
}

/// Every valid `(instance, value)` of a (possibly wildcard) counter.
fn read_array(counter: PDH_HCOUNTER) -> Vec<(String, f64)> {
    // Instances can appear between the size query and the read, so retry.
    for _ in 0..3 {
        let (mut bytes, mut count) = (0u32, 0u32);
        // SAFETY: size query with no buffer.
        let status = unsafe {
            PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, None)
        };
        if status != PDH_MORE_DATA || bytes == 0 {
            return Vec::new();
        }
        let item = size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        let mut buf = vec![PDH_FMT_COUNTERVALUE_ITEM_W::default(); (bytes as usize).div_ceil(item)];
        // SAFETY: `buf` holds at least `bytes` bytes and is suitably aligned.
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut bytes,
                &mut count,
                Some(buf.as_mut_ptr()),
            )
        };
        if status == PDH_MORE_DATA {
            continue;
        }
        if status != 0 {
            return Vec::new();
        }
        return buf
            .iter()
            .take(count as usize)
            .filter(|i| {
                matches!(
                    i.FmtValue.CStatus,
                    PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA
                )
            })
            .map(|i| {
                // SAFETY: names point into `buf`; PDH_FMT_DOUBLE fills `doubleValue`.
                let name = unsafe { i.szName.to_string() }.unwrap_or_default();
                let value = unsafe { i.FmtValue.Anonymous.doubleValue };
                (name, value)
            })
            .collect();
    }
    Vec::new()
}
