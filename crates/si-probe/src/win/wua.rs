//! Driver updates Windows Update offers but has not installed, via the
//! Windows Update Agent COM API. Slow (10–60 s) and needs network: run it on
//! its own thread.

use si_core::drivers::AvailableDriverUpdate;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::UpdateAgent::{
    IUpdateCollection, IUpdateSession, IWindowsDriverUpdate, UpdateCollection, UpdateSession,
    orcSucceeded, orcSucceededWithErrors,
};
use windows::core::{BSTR, Interface};

use crate::ProbeError;
use crate::install::InstallReport;
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
                update_id: update
                    .Identity()
                    .and_then(|i| i.UpdateID())
                    .map(|b| b.to_string())
                    .unwrap_or_default(),
            });
        }
        Ok(found)
    }
}

/// Downloads and installs the offered driver updates whose IDs are in
/// `update_ids`. Needs administrator rights (runs in the elevated helper).
pub fn install_drivers(update_ids: &[String]) -> Result<InstallReport, ProbeError> {
    // SAFETY: initialises COM for this thread; balanced below when it succeeded.
    let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
    let result = install(update_ids).map_err(|e| ProbeError::Windows(e.message()));
    if initialized {
        // SAFETY: matches the successful CoInitializeEx above.
        unsafe { CoUninitialize() };
    }
    result
}

fn install(update_ids: &[String]) -> windows::core::Result<InstallReport> {
    let mut report = InstallReport::default();
    // SAFETY: standard COM activation and calls on owned interfaces.
    unsafe {
        let session: IUpdateSession = CoCreateInstance(&UpdateSession, None, CLSCTX_INPROC_SERVER)?;
        let offered = session
            .CreateUpdateSearcher()?
            .Search(&BSTR::from(DRIVER_SEARCH))?
            .Updates()?;
        let selected: IUpdateCollection =
            CoCreateInstance(&UpdateCollection, None, CLSCTX_INPROC_SERVER)?;
        let mut titles = Vec::new();
        let mut found = 0;
        for index in 0..offered.Count()? {
            let update = offered.get_Item(index)?;
            let id = update.Identity()?.UpdateID()?.to_string();
            if !update_ids
                .iter()
                .any(|wanted| wanted.eq_ignore_ascii_case(&id))
            {
                continue;
            }
            found += 1;
            if !update.EulaAccepted()?.as_bool() {
                update.AcceptEula()?;
            }
            selected.Add(&update)?;
            titles.push(update.Title()?.to_string());
        }
        // Requested updates Windows Update no longer offers count as failed.
        for _ in found..update_ids.len() {
            report
                .failed
                .push("An update is no longer offered by Windows Update".into());
        }
        if titles.is_empty() {
            return Ok(report);
        }

        let downloader = session.CreateUpdateDownloader()?;
        downloader.SetUpdates(&selected)?;
        downloader.Download()?;

        let installer = session.CreateUpdateInstaller()?;
        installer.SetUpdates(&selected)?;
        let result = installer.Install()?;
        report.reboot_required = result.RebootRequired()?.as_bool();
        for (index, title) in titles.into_iter().enumerate() {
            let code = result.GetUpdateResult(index as i32)?.ResultCode()?;
            if code == orcSucceeded || code == orcSucceededWithErrors {
                report.installed.push(title);
            } else {
                report.failed.push(title);
            }
        }
    }
    Ok(report)
}
