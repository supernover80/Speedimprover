use si_probe::temps::*;
fn s(id: &str, name: &str, t: &str, v: f32) -> LhmSensor {
    LhmSensor {
        identifier: Some(id.into()),
        name: Some(name.into()),
        sensor_type: Some(t.into()),
        value: Some(v),
    }
}
#[test]
fn picks_cpu_package() {
    let rows = [
        s(
            "/intelcpu/0/temperature/0",
            "CPU Core #1",
            "Temperature",
            70.0,
        ),
        s(
            "/intelcpu/0/temperature/8",
            "CPU Package",
            "Temperature",
            74.0,
        ),
        s("/intelcpu/0/load/0", "CPU Total", "Load", 99.0),
    ];
    assert_eq!(cpu_temp_from_sensors(&rows), Some(74.0));
}
#[test]
fn amd_tctl_and_fallback_max() {
    assert_eq!(
        cpu_temp_from_sensors(&[s(
            "/amdcpu/0/temperature/2",
            "Core (Tctl/Tdie)",
            "Temperature",
            81.5
        )]),
        Some(81.5)
    );
    assert_eq!(
        cpu_temp_from_sensors(&[
            s("/amdcpu/0/temperature/3", "CCD1", "Temperature", 60.0),
            s("/amdcpu/0/temperature/4", "CCD2", "Temperature", 66.0)
        ]),
        Some(66.0)
    );
}
#[test]
fn picks_gpu_core() {
    let rows = [
        s(
            "/gpu-nvidia/0/temperature/0",
            "GPU Core",
            "Temperature",
            68.0,
        ),
        s(
            "/gpu-nvidia/0/temperature/2",
            "GPU Hot Spot",
            "Temperature",
            80.0,
        ),
    ];
    assert_eq!(gpu_temp_from_sensors(&rows), Some(68.0));
}
#[test]
fn thermal_zone_kelvin() {
    assert_eq!(
        thermal_zone_celsius(&[("\\_TZ.TZ00".into(), 300.15), ("\\_TZ.TZ01".into(), 345.15)]),
        Some(72.0)
    );
    assert_eq!(thermal_zone_celsius(&[("z".into(), 0.0)]), None);
}
#[test]
fn temps_absent_are_none() {
    assert_eq!(cpu_temp_from_sensors(&[]), None);
    assert_eq!(gpu_temp_from_sensors(&[]), None);
    assert_eq!(deci_celsius(0), None);
    assert_eq!(deci_celsius(655), Some(65.5));
}
