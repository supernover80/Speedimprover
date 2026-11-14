//! The driver version catalog: fetched from the project's GitHub copy, with
//! the copy compiled into the binary as the offline fallback.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use si_core::drivers::DriverCatalog;

use crate::ProbeError;

const SUPPORTED_SCHEMA: u32 = 1;
const FETCH_TIMEOUT: Duration = Duration::from_secs(10);
/// Generic client name: no user or machine details leave the PC.
const USER_AGENT: &str = concat!("SpeedImprover/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogSource {
    Remote,
    Bundled,
}

/// Downloads and validates the catalog. Errors on non-200 responses, bad
/// JSON or an unsupported schema.
pub fn fetch_catalog(url: &str, timeout: Duration) -> Result<DriverCatalog, ProbeError> {
    let net = |e: reqwest::Error| ProbeError::Network(e.to_string());
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .user_agent(USER_AGENT)
        .build()
        .map_err(net)?;
    let response = client.get(url).send().map_err(net)?;
    let status = response.status();
    if status != reqwest::StatusCode::OK {
        return Err(ProbeError::Network(format!("HTTP {status} from {url}")));
    }
    let text = response.text().map_err(net)?;
    let catalog = DriverCatalog::parse(&text).map_err(|e| ProbeError::Network(e.to_string()))?;
    if catalog.schema != SUPPORTED_SCHEMA {
        return Err(ProbeError::Network(format!(
            "unsupported catalog schema {}",
            catalog.schema
        )));
    }
    Ok(catalog)
}

/// The online catalog if reachable and valid, else the bundled one.
pub fn load_catalog(url: &str) -> (DriverCatalog, CatalogSource) {
    match fetch_catalog(url, FETCH_TIMEOUT) {
        Ok(catalog) => (catalog, CatalogSource::Remote),
        Err(e) => {
            tracing::info!("using bundled driver catalog: {e}");
            (DriverCatalog::bundled(), CatalogSource::Bundled)
        }
    }
}
