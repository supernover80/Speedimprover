//! System Restore points around driver installs (needs administrator rights;
//! silently skipped when System Restore is turned off).

use windows::Win32::System::Restore::{
    BEGIN_SYSTEM_CHANGE, DEVICE_DRIVER_INSTALL, END_SYSTEM_CHANGE, RESTOREPOINTINFOW,
    SRSetRestorePointW, STATEMGRSTATUS,
};

fn info(
    event: windows::Win32::System::Restore::RESTOREPOINTINFO_EVENT_TYPE,
    sequence: i64,
) -> RESTOREPOINTINFOW {
    // The struct is packed: build the text separately and move it in whole.
    let mut description = [0u16; 256];
    for (slot, unit) in description
        .iter_mut()
        .zip("SpeedImprover driver update".encode_utf16())
    {
        *slot = unit;
    }
    RESTOREPOINTINFOW {
        dwEventType: event,
        dwRestorePtType: DEVICE_DRIVER_INSTALL,
        llSequenceNumber: sequence,
        szDescription: description,
    }
}

/// Starts a restore point; returns its sequence number.
pub fn begin() -> Option<i64> {
    let mut status = STATEMGRSTATUS::default();
    // SAFETY: both structs are valid for the duration of the call.
    let ok = unsafe { SRSetRestorePointW(&info(BEGIN_SYSTEM_CHANGE, 0), &mut status) };
    ok.as_bool().then_some(status.llSequenceNumber)
}

/// Finishes the restore point started by [`begin`].
pub fn end(sequence: i64) {
    let mut status = STATEMGRSTATUS::default();
    // SAFETY: as above.
    let _ = unsafe { SRSetRestorePointW(&info(END_SYSTEM_CHANGE, sequence), &mut status) };
}
