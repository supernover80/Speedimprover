use si_core::{fixtures, model::*, tier::*};

fn gpu(name: &str, vram: u64, integrated: bool) -> Tier {
    gpu_tier(
        &GpuInfo {
            name: name.into(),
            vram_mb: vram,
            is_integrated: integrated,
            ..Default::default()
        },
        &TierTable::bundled_gpu(),
    )
}
fn cpu(name: &str, cores: u32) -> Tier {
    cpu_tier(
        &CpuInfo {
            name: name.into(),
            physical_cores: cores,
            ..Default::default()
        },
        &TierTable::bundled_cpu(),
    )
}

#[test]
fn highest_matching_tier_wins() {
    assert_eq!(gpu("NVIDIA GeForce RTX 4070 Ti", 12282, false), Tier::Ultra);
    assert_eq!(gpu("NVIDIA GeForce RTX 4070", 12282, false), Tier::High);
    assert_eq!(gpu("AMD Radeon RX 5700 XT", 8176, false), Tier::Mid);
    assert_eq!(gpu("Radeon RX 570 Series", 4096, false), Tier::Low);
    assert_eq!(
        gpu("Intel(R) Arc(TM) A770 Graphics", 16256, false),
        Tier::Mid
    ); // (R)/(TM) stripped before matching
}
#[test]
fn mobile_gpus_step_down() {
    assert_eq!(
        gpu("NVIDIA GeForce RTX 4070 Laptop GPU", 8188, false),
        Tier::Mid
    );
    assert_eq!(gpu("AMD Radeon RX 6800M", 12272, false), Tier::Mid);
}
#[test]
fn unknown_gpu_falls_back() {
    assert_eq!(gpu("Mystery GPU", 12288, false), Tier::High);
    assert_eq!(gpu("Mystery GPU", 8192, false), Tier::Mid);
    assert_eq!(gpu("Mystery GPU", 4096, false), Tier::Low);
    assert_eq!(gpu("Mystery iGPU", 16384, true), Tier::Low);
}
#[test]
fn cpu_tiers() {
    assert_eq!(cpu("AMD Ryzen 7 7800X3D 8-Core Processor", 8), Tier::Ultra);
    assert_eq!(cpu("12th Gen Intel(R) Core(TM) i5-1235U", 10), Tier::Low);
    assert_eq!(cpu("Mystery CPU", 8), Tier::Mid);
    assert_eq!(cpu("Mystery CPU", 4), Tier::Low);
}
#[test]
fn overall_rules() {
    assert_eq!(overall_tier(Tier::Ultra, Tier::Mid, 32_000), Tier::High);
    assert_eq!(overall_tier(Tier::Ultra, Tier::High, 32_000), Tier::Ultra);
    assert_eq!(overall_tier(Tier::High, Tier::High, 8_000), Tier::Mid);
    assert_eq!(overall_tier(Tier::High, Tier::High, 6_000), Tier::Low);
}
#[test]
fn hybrid_laptop_uses_discrete() {
    let mut s = fixtures::struggling_laptop();
    s.gpus.push(GpuInfo {
        name: "NVIDIA GeForce RTX 4060 Laptop GPU".into(),
        vendor: Vendor::Nvidia,
        vram_mb: 8188,
        ..Default::default()
    });
    let hw = hardware_profile(&s);
    assert_eq!((hw.gpu, hw.vram_mb), (Tier::Low, 8188)); // RTX 4060 = Mid, Laptop ⇒ Low
}
#[test]
fn fixture_profiles() {
    let hw = hardware_profile(&fixtures::healthy_desktop());
    assert_eq!(
        (hw.gpu, hw.cpu, hw.overall),
        (Tier::High, Tier::Ultra, Tier::High)
    );
    assert_eq!(
        hardware_profile(&SystemSnapshot::default()).overall,
        Tier::Low
    );
}
