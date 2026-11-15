use std::path::{Path, PathBuf};

use si_probe::install::*;

const ID: &str = "0b5b8d3c-1a2b-4c5d-8e9f-001122334455";

#[test]
fn helper_args_round_trip() {
    let ids = vec![ID.to_string()];
    let result = Path::new("C:\\Temp\\speedimprover-install-42.json");
    let args = helper_args(result, &ids);
    assert_eq!(
        parse_helper_args(&args),
        Some((
            PathBuf::from("C:\\Temp\\speedimprover-install-42.json"),
            ids
        ))
    );
}

#[test]
fn install_args_reject_non_guid() {
    let path = "C:\\Temp\\speedimprover-install-1.json".to_string();
    for bad in [
        "; calc",
        "0b5b8d3c-1a2b-4c5d-8e9f-00112233445",
        "../../x",
        "",
    ] {
        assert!(!is_update_id(bad), "{bad}");
        assert_eq!(
            parse_helper_args(&["--install-drivers".into(), path.clone(), bad.into()]),
            None
        );
    }
    assert!(is_update_id(ID));
    assert_eq!(parse_helper_args(&["--install-drivers".into(), path]), None);
}

#[test]
fn install_args_reject_other_result_files() {
    for bad in [
        "C:\\Windows\\System32\\drivers\\etc\\hosts",
        "C:\\Temp\\report.json",
    ] {
        assert_eq!(
            parse_helper_args(&["--install-drivers".into(), bad.into(), ID.into()]),
            None,
            "{bad}"
        );
    }
}

#[test]
fn not_windows_install_is_an_error() {
    #[cfg(not(windows))]
    assert!(si_probe::install_driver_updates_elevated(&[ID.to_string()]).is_err());
}
