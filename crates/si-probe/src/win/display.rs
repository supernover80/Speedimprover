//! Monitors attached to the desktop: resolution, current and maximum refresh rate.

use si_core::model::{DisplayInfo, SystemSnapshot};
use windows::Win32::Graphics::Gdi::{
    DEVMODEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS,
    ENUM_DISPLAY_SETTINGS_MODE, EnumDisplayDevicesW, EnumDisplaySettingsW,
};
use windows::core::PCWSTR;

use crate::winsettings::max_hz_for_resolution;

fn wide_to_string(w: &[u16]) -> String {
    let len = w.iter().position(|&c| c == 0).unwrap_or(w.len());
    String::from_utf16_lossy(&w[..len])
}

fn new_devmode() -> DEVMODEW {
    DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        ..Default::default()
    }
}

pub fn collect_displays(snap: &mut SystemSnapshot) {
    snap.displays.clear();
    for index in 0.. {
        let mut device = DISPLAY_DEVICEW {
            cb: size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        // SAFETY: `device.cb` is set; a null device name enumerates adapters.
        if !unsafe { EnumDisplayDevicesW(PCWSTR::null(), index, &mut device, 0) }.as_bool() {
            break;
        }
        if (device.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP).0 == 0 {
            continue;
        }
        let name = PCWSTR(device.DeviceName.as_ptr());
        let mut current = new_devmode();
        // SAFETY: `name` points into `device`, alive for the call; `dmSize` is set.
        if !unsafe { EnumDisplaySettingsW(name, ENUM_CURRENT_SETTINGS, &mut current) }.as_bool() {
            continue;
        }
        let mut modes = Vec::new();
        for mode in 0.. {
            let mut dm = new_devmode();
            // SAFETY: as above; enumeration ends when the call returns FALSE.
            if !unsafe { EnumDisplaySettingsW(name, ENUM_DISPLAY_SETTINGS_MODE(mode), &mut dm) }
                .as_bool()
            {
                break;
            }
            modes.push((dm.dmPelsWidth, dm.dmPelsHeight, dm.dmDisplayFrequency));
        }
        let (width, height, hz) = (
            current.dmPelsWidth,
            current.dmPelsHeight,
            current.dmDisplayFrequency,
        );
        snap.displays.push(DisplayInfo {
            name: wide_to_string(&device.DeviceName),
            width,
            height,
            current_hz: hz,
            max_hz: max_hz_for_resolution(&modes, width, height).max(hz),
        });
    }
}
