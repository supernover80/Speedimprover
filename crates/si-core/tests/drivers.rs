use si_core::{drivers::*, fixtures, model::DriverInfo};
use std::cmp::Ordering::*;

fn catalog(vendor: &str, class: &str, latest: &str) -> DriverCatalog {
    DriverCatalog::parse(&format!(r#"{{"schema":1,"updated":"2026-11-01","entries":[{{"vendor":"{vendor}","class":"{class}","latest_version":"{latest}","download_url":"https://example.test/dl"}}]}}"#)).unwrap()
}
fn status_of<'a>(rows: &'a [DriverRow], name: &str) -> &'a DriverStatus {
    &rows
        .iter()
        .find(|r| r.driver.device_name == name)
        .unwrap()
        .status
}

#[test]
fn compares_dotted_versions() {
    assert_eq!(compare_versions("32.0.15.6094", "32.0.15.6100"), Some(Less));
    assert_eq!(compare_versions("1.2", "1.2.0"), Some(Equal));
    assert_eq!(compare_versions("abc", "1.0"), None);
    assert_eq!(compare_versions("", "1.0"), None);
}
#[test]
fn converts_nvidia_versions() {
    assert_eq!(
        nvidia_display_version("32.0.15.6094").as_deref(),
        Some("560.94")
    );
    assert_eq!(
        nvidia_display_version("31.0.15.5222").as_deref(),
        Some("552.22")
    );
    assert_eq!(nvidia_display_version("1.2"), None);
}
#[test]
fn detects_vendors() {
    assert_eq!(driver_vendor("NVIDIA"), Some(DriverVendor::Nvidia));
    assert_eq!(
        driver_vendor("Advanced Micro Devices, Inc."),
        Some(DriverVendor::Amd)
    );
    assert_eq!(
        driver_vendor("Intel Corporation"),
        Some(DriverVendor::Intel)
    );
    assert_eq!(driver_vendor("Realtek"), None);
}
#[test]
fn windows_update_match_wins() {
    let wu = [AvailableDriverUpdate {
        hardware_id: "pci\\ven_8086&dev_46a8".into(),
        ..Default::default()
    }];
    let rows = evaluate_drivers(&fixtures::struggling_laptop(), &DriverCatalog::empty(), &wu);
    assert_eq!(
        status_of(&rows, "Intel(R) Iris(R) Xe Graphics"),
        &DriverStatus::UpdateAvailable {
            source: UpdateSource::WindowsUpdate,
            latest: None,
            url: "ms-settings:windowsupdate-optionalupdates".into()
        }
    );
}
#[test]
fn catalog_compares_nvidia_format() {
    let s = fixtures::healthy_desktop();
    let rows = evaluate_drivers(&s, &catalog("nvidia", "Display", "571.96"), &[]);
    assert!(matches!(status_of(&rows, "NVIDIA GeForce RTX 4070 SUPER"),
        DriverStatus::UpdateAvailable { source: UpdateSource::Catalog, latest: Some(v), .. } if v == "571.96"));
    let rows = evaluate_drivers(&s, &catalog("nvidia", "Display", "566.14"), &[]);
    assert_eq!(
        status_of(&rows, "NVIDIA GeForce RTX 4070 SUPER"),
        &DriverStatus::NoUpdateFound
    );
}
#[test]
fn old_driver_without_catalog_is_possibly_outdated() {
    let rows = evaluate_drivers(&fixtures::struggling_laptop(), &DriverCatalog::empty(), &[]);
    assert_eq!(
        status_of(&rows, "Intel(R) Iris(R) Xe Graphics"),
        &DriverStatus::PossiblyOutdated { age_days: 883 }
    );
}
#[test]
fn unparseable_version_skips_catalog() {
    let mut s = fixtures::healthy_desktop();
    s.drivers[0].version = "n/a".into();
    let rows = evaluate_drivers(&s, &catalog("nvidia", "Display", "571.96"), &[]);
    assert_eq!(
        status_of(&rows, "NVIDIA GeForce RTX 4070 SUPER"),
        &DriverStatus::NoUpdateFound
    );
}
#[test]
fn rows_filtered_and_sorted() {
    let mut s = fixtures::struggling_laptop();
    let sys = |m: &str, n: &str| DriverInfo {
        device_name: n.into(),
        class: "System".into(),
        manufacturer: m.into(),
        version: "1.0".into(),
        ..Default::default()
    };
    s.drivers.push(sys("Realtek", "Realtek Card Reader"));
    s.drivers
        .push(sys("Intel Corporation", "Intel Chipset SMBus"));
    let names: Vec<_> = evaluate_drivers(&s, &DriverCatalog::empty(), &[])
        .into_iter()
        .map(|r| r.driver.device_name)
        .collect();
    assert_eq!(
        names,
        [
            "Intel(R) Iris(R) Xe Graphics",
            "Intel(R) Wi-Fi 6 AX201 160MHz",
            "Intel Chipset SMBus"
        ]
    );
}
#[test]
fn bundled_catalog_parses() {
    assert_eq!(DriverCatalog::bundled().schema, 1);
}
