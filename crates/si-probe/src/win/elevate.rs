//! Starts this program again with administrator rights (UAC prompt) to
//! install drivers, waits for it, and reads its report.

use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED};
use windows::Win32::System::Threading::{INFINITE, WaitForSingleObject};
use windows::Win32::UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
use windows::core::{HSTRING, PCWSTR, w};

use crate::ProbeError;
use crate::install::{InstallReport, RESULT_PREFIX, helper_args};

const SW_HIDE: i32 = 0;

pub fn run_elevated_install(update_ids: &[String]) -> Result<InstallReport, ProbeError> {
    let exe = std::env::current_exe()?;
    let result = std::env::temp_dir().join(format!("{RESULT_PREFIX}{}.json", std::process::id()));
    let _ = std::fs::remove_file(&result);
    let parameters = helper_args(&result, update_ids)
        .iter()
        .map(|a| format!("\"{a}\""))
        .collect::<Vec<_>>()
        .join(" ");
    let file = HSTRING::from(exe.as_os_str());
    let parameters = HSTRING::from(parameters);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(parameters.as_ptr()),
        nShow: SW_HIDE,
        ..Default::default()
    };
    // SAFETY: `info` and the strings it points to outlive the call.
    if let Err(e) = unsafe { ShellExecuteExW(&mut info) } {
        let cancelled = e.code() == windows::core::HRESULT::from_win32(ERROR_CANCELLED.0);
        return Err(ProbeError::Windows(if cancelled {
            "Installation cancelled".into()
        } else {
            format!("Could not start the installer: {}", e.message())
        }));
    }
    if !info.hProcess.is_invalid() {
        // SAFETY: valid process handle returned because of SEE_MASK_NOCLOSEPROCESS.
        unsafe {
            WaitForSingleObject(info.hProcess, INFINITE);
            let _ = CloseHandle(info.hProcess);
        }
    }
    let text = std::fs::read_to_string(&result)
        .map_err(|_| ProbeError::Windows("The installer finished without a report".into()))?;
    let _ = std::fs::remove_file(&result);
    Ok(serde_json::from_str(&text)?)
}
