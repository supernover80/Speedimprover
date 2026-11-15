//! Installing Windows Update drivers.
//!
//! Installing needs administrator rights, so the app starts itself again
//! with `--install-drivers <result.json> <update-id>…` through a UAC prompt.
//! That helper only accepts Windows Update IDs (GUIDs), installs exactly
//! those offered updates, writes an [`InstallReport`] and exits.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const INSTALL_FLAG: &str = "--install-drivers";
/// The helper only writes result files named like this.
pub const RESULT_PREFIX: &str = "speedimprover-install-";
const RESULT_SUFFIX: &str = ".json";

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct InstallReport {
    /// Titles of updates that installed.
    pub installed: Vec<String>,
    /// Titles (or error messages) of updates that did not install.
    pub failed: Vec<String>,
    pub reboot_required: bool,
    pub restore_point_created: bool,
}

/// A Windows Update ID: a GUID like `0b5b8d3c-1a2b-4c5d-8e9f-001122334455`.
pub fn is_update_id(s: &str) -> bool {
    s.len() == 36
        && s.char_indices().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        })
}

/// File name of `path`, splitting on both `/` and `\` (paths come from Windows).
fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn is_result_file(path: &str) -> bool {
    let name = file_name(path);
    name.starts_with(RESULT_PREFIX) && name.ends_with(RESULT_SUFFIX)
}

/// Arguments for the elevated helper (without the program name).
pub fn helper_args(result: &Path, ids: &[String]) -> Vec<String> {
    let mut args = vec![
        INSTALL_FLAG.to_string(),
        result.to_string_lossy().into_owned(),
    ];
    args.extend(ids.iter().cloned());
    args
}

/// The helper's result path and update IDs, if `args` asks for an install and
/// every value is valid. Anything unexpected ⇒ `None` (nothing is installed).
pub fn parse_helper_args(args: &[String]) -> Option<(PathBuf, Vec<String>)> {
    let flag = args.iter().position(|a| a == INSTALL_FLAG)?;
    let result = args.get(flag + 1)?;
    let ids = &args[flag + 2..];
    if !is_result_file(result) || ids.is_empty() || !ids.iter().all(|id| is_update_id(id)) {
        return None;
    }
    Some((PathBuf::from(result), ids.to_vec()))
}
