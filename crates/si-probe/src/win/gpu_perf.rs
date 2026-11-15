//! GPU temperature from the graphics kernel's adapter performance data — the
//! source Task Manager uses. Works for any vendor whose driver reports it
//! (WDDM 2.4+), without administrator rights.

use std::ffi::c_void;

use windows::Wdk::Graphics::Direct3D::{
    D3DKMT_ADAPTER_PERFDATA, D3DKMT_CLOSEADAPTER, D3DKMT_OPENADAPTERFROMLUID,
    D3DKMT_QUERYADAPTERINFO, D3DKMTCloseAdapter, D3DKMTOpenAdapterFromLuid, D3DKMTQueryAdapterInfo,
    KMTQAITYPE_ADAPTERPERFDATA,
};
use windows::Win32::Foundation::LUID;

use crate::temps::deci_celsius;

/// Temperature (°C) of the adapter with this LUID, if its driver reports one.
pub fn gpu_temperature(luid: u64) -> Option<f32> {
    let mut open = D3DKMT_OPENADAPTERFROMLUID {
        AdapterLuid: LUID {
            LowPart: luid as u32,
            HighPart: (luid >> 32) as i32,
        },
        hAdapter: 0,
    };
    // SAFETY: `open` is a valid in/out struct for the call.
    if unsafe { D3DKMTOpenAdapterFromLuid(&mut open) }.is_err() {
        return None;
    }
    let mut perf = D3DKMT_ADAPTER_PERFDATA::default();
    let mut query = D3DKMT_QUERYADAPTERINFO {
        hAdapter: open.hAdapter,
        Type: KMTQAITYPE_ADAPTERPERFDATA,
        pPrivateDriverData: &mut perf as *mut D3DKMT_ADAPTER_PERFDATA as *mut c_void,
        PrivateDriverDataSize: size_of::<D3DKMT_ADAPTER_PERFDATA>() as u32,
    };
    // SAFETY: `query` points at `perf`, which lives until after the call.
    let status = unsafe { D3DKMTQueryAdapterInfo(&mut query) };
    let close = D3DKMT_CLOSEADAPTER {
        hAdapter: open.hAdapter,
    };
    // SAFETY: closes the handle opened above, exactly once.
    let _ = unsafe { D3DKMTCloseAdapter(&close) };
    if status.is_err() {
        return None;
    }
    deci_celsius(perf.Temperature)
}
