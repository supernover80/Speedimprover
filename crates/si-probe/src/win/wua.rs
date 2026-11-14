//! Driver updates Windows Update offers but has not installed, via the
//! Windows Update Agent COM API. Slow (10–60 s) and needs network: run it on
//! its own thread.

use si_core::drivers::AvailableDriverUpdate;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::UpdateAgent::{IUpdateSession, IWindowsDriverUpdate, UpdateSession};
use windows::core::{BSTR, Interface};

use crate::ProbeError;
use crate::mapping::ole_date_to_ymd;

const DRIVER_SEARCH: &str = "IsInstalled=0 and Type='Driver'";

pub fn search_drivers() -> Result<Vec<AvailableDriverUpdate>, ProbeError> {
    // SAFETY: initialises COM for this thread; balanced below when it succeeded.
    let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
    let result = search().map_err(|e| ProbeError::Windows(e.message()));
    if initialized {
        // SAFETY: matches the successful CoInitializeEx above.
        unsafe { CoUninitialize() };
    }
    result
}

fn search() -> windows::core::Result<Vec<AvailableDriverUpdate>> {
    // SAFETY: standard COM activation and property reads on owned interfaces.
    unsafe {
        let session: IUpdateSession = CoCreateInstance(&UpdateSession, None, CLSCTX_INPROC_SERVER)?;
        let searcher = session.CreateUpdateSearcher()?;
        let updates = searcher.Search(&BSTR::from(DRIVER_SEARCH))?.Updates()?;
        let mut found = Vec::new();
        for index in 0..updates.Count()? {
            let update = updates.get_Item(index)?;
            let Ok(driver) = update.cast::<IWindowsDriverUpdate>() else {
                continue;
            };
            let text =
                |v: windows::core::Result<BSTR>| v.map(|b| b.to_string()).unwrap_or_default();
            found.push(AvailableDriverUpdate {
                title: text(update.Title()),
                hardware_id: text(driver.DriverHardwareID()),
                driver_class: text(driver.DriverClass()),
                driver_model: text(driver.DriverModel()),
                ver_date: driver.DriverVerDate().ok().and_then(ole_date_to_ymd),
            });
        }
        Ok(found)
    }
}
