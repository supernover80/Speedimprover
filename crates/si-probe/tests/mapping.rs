use si_core::model::*;
use si_probe::mapping::*;

#[test]
fn parses_wmi_and_ole_dates() {
    assert_eq!(
        parse_wmi_date("20240115000000.000000-000").as_deref(),
        Some("2024-01-15")
    );
    assert_eq!(parse_wmi_date("garbage"), None);
    assert_eq!(ole_date_to_ymd(45658.0).as_deref(), Some("2025-01-01"));
}
#[test]
fn ddr5_module_maps() {
    let m = memory_module(&Win32PhysicalMemory {
        capacity: Some("17179869184".into()),
        speed: Some(6000),
        configured_clock_speed: Some(4800),
        smbios_memory_type: Some(34),
    });
    assert_eq!(
        m,
        MemoryModule {
            capacity_mb: 16384,
            rated_mhz: 6000,
            configured_mhz: 4800,
            generation: MemoryGen::Ddr5
        }
    );
}
#[test]
fn skips_microsoft_and_software_adapters() {
    assert!(is_skipped_adapter(0x1414, 0));
    assert!(is_skipped_adapter(0x10DE, 2));
    assert!(!is_skipped_adapter(0x10DE, 0));
    assert_eq!(vendor_from_pci_id(0x1002), Vendor::Amd);
}
#[test]
fn integrated_names() {
    for n in [
        "Intel(R) UHD Graphics 770",
        "Intel(R) Iris(R) Xe Graphics",
        "AMD Radeon(TM) Graphics",
        "AMD Radeon 780M Graphics",
    ] {
        assert!(integrated_by_name(n), "{n}");
    }
    assert!(!integrated_by_name("AMD Radeon RX 7800 XT"));
}
#[test]
fn disk_media_by_letter() {
    assert_eq!(disk_media(4, 17), DiskMedia::Nvme);
    assert_eq!(disk_media(3, 11), DiskMedia::Hdd);
    let disks = [MsftPhysicalDisk {
        device_id: Some("1".into()),
        media_type: Some(3),
        bus_type: Some(11),
    }];
    let parts = [
        MsftPartition {
            disk_number: Some(1),
            drive_letter: Some('D' as u16),
        },
        MsftPartition {
            disk_number: Some(1),
            drive_letter: Some(0),
        },
    ];
    assert_eq!(
        media_by_letter(&disks, &parts).get(&'D'),
        Some(&DiskMedia::Hdd)
    );
}
#[test]
fn drivers_map_and_skip_incomplete() {
    let row = Win32PnPSignedDriver {
        device_name: Some("NVIDIA GeForce RTX 4070".into()),
        device_class: Some("DISPLAY".into()),
        manufacturer: Some("NVIDIA".into()),
        driver_version: Some("32.0.15.6094".into()),
        driver_date: Some("20240910000000.******+***".into()),
        hardware_id: Some("PCI\\VEN_10DE&DEV_2786".into()),
    };
    let d = driver_info(&row).unwrap();
    assert_eq!(
        (d.class.as_str(), d.date.as_deref()),
        ("Display", Some("2024-09-10"))
    );
    assert!(
        driver_info(&Win32PnPSignedDriver {
            driver_version: None,
            ..row
        })
        .is_none()
    );
}
#[test]
fn attaches_gpu_driver_by_name() {
    let mut g = [GpuInfo {
        name: "NVIDIA GeForce RTX 4070".into(),
        ..Default::default()
    }];
    attach_gpu_drivers(
        &mut g,
        &[Win32VideoController {
            name: Some("nvidia geforce rtx 4070 ".into()),
            driver_version: Some("32.0.15.6094".into()),
            driver_date: Some("20240910000000.000000-000".into()),
        }],
    );
    assert_eq!(g[0].driver_date.as_deref(), Some("2024-09-10"));
}
#[test]
fn memory_integrity_flag() {
    assert_eq!(
        memory_integrity_on(&Win32DeviceGuard {
            security_services_running: Some(vec![1, 2])
        }),
        Some(true)
    );
    assert_eq!(
        memory_integrity_on(&Win32DeviceGuard {
            security_services_running: None
        }),
        None
    );
}
