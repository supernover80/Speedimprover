//! LibreHardwareMonitor / OpenHardwareMonitor sensors over WMI. These exist
//! only while the user runs one of those apps; when neither answers we wait
//! 30 s before asking again.

use std::time::{Duration, Instant};

use wmi::WMIConnection;

use crate::temps::LhmSensor;

const NAMESPACES: [&str; 2] = [r"ROOT\LibreHardwareMonitor", r"ROOT\OpenHardwareMonitor"];
const RETRY_AFTER: Duration = Duration::from_secs(30);
const QUERY: &str =
    "SELECT Identifier, Name, SensorType, Value FROM Sensor WHERE SensorType = 'Temperature'";

#[derive(Default)]
pub struct LhmReader {
    next_attempt: Option<Instant>,
}

impl LhmReader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Temperature sensors, or empty when no monitoring app is running.
    pub fn read(&mut self) -> Vec<LhmSensor> {
        if self.next_attempt.is_some_and(|at| Instant::now() < at) {
            return Vec::new();
        }
        for namespace in NAMESPACES {
            let rows = WMIConnection::with_namespace_path(namespace)
                .and_then(|c| c.raw_query::<LhmSensor>(QUERY));
            if let Ok(rows) = rows {
                if !rows.is_empty() {
                    self.next_attempt = None;
                    return rows;
                }
            }
        }
        self.next_attempt = Some(Instant::now() + RETRY_AFTER);
        Vec::new()
    }
}
