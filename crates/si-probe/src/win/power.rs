//! Battery state and the active power plan.

use std::ptr::null_mut;

use si_core::model::SystemSnapshot;
use windows::Win32::Foundation::{ERROR_SUCCESS, HLOCAL, LocalFree};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, PowerGetActiveScheme, SYSTEM_POWER_STATUS,
};
use windows::core::GUID;

use crate::winsettings::{battery_state, power_plan_from_guid};
use crate::{ScanStep, ScanWarning};

/// Laptop / on-battery flags.
pub fn collect_battery(snap: &mut SystemSnapshot, warnings: &mut Vec<ScanWarning>) {
    let mut status = SYSTEM_POWER_STATUS::default();
    // SAFETY: `status` is a valid out-pointer for the call's duration.
    match unsafe { GetSystemPowerStatus(&mut status) } {
        Ok(()) => {
            let (is_laptop, on_battery) = battery_state(status.BatteryFlag, status.ACLineStatus);
            snap.machine.is_laptop = is_laptop;
            snap.machine.on_battery = on_battery;
        }
        Err(e) => warnings.push(ScanWarning::new(
            ScanStep::Hardware,
            format!("power status unavailable: {e}"),
        )),
    }
}

fn guid_string(g: &GUID) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        g.data1,
        g.data2,
        g.data3,
        g.data4[0],
        g.data4[1],
        g.data4[2],
        g.data4[3],
        g.data4[4],
        g.data4[5],
        g.data4[6],
        g.data4[7]
    )
}

/// Active power plan, identified by GUID.
pub fn collect_power_plan(snap: &mut SystemSnapshot, warnings: &mut Vec<ScanWarning>) {
    let mut scheme: *mut GUID = null_mut();
    // SAFETY: on success Windows allocates the GUID; it is freed with LocalFree below.
    let rc = unsafe { PowerGetActiveScheme(None, &mut scheme) };
    if rc != ERROR_SUCCESS || scheme.is_null() {
        warnings.push(ScanWarning::new(
            ScanStep::OsSettings,
            format!("power plan unavailable (error {})", rc.0),
        ));
        return;
    }
    // SAFETY: `scheme` is non-null and points to a GUID owned by us.
    let guid = unsafe { *scheme };
    // SAFETY: `scheme` was allocated by PowerGetActiveScheme with LocalAlloc.
    unsafe { LocalFree(Some(HLOCAL(scheme.cast()))) };
    snap.os.power_plan = power_plan_from_guid(&guid_string(&guid));
}
