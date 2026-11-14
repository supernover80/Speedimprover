//! WMI collectors. Every query goes through [`query_in`], so a missing
//! namespace or class (Windows Home, no admin rights, broken WMI repository)
//! only adds a warning.

use serde::de::DeserializeOwned;
use si_core::model::SystemSnapshot;
use wmi::WMIConnection;

use crate::mapping::{
    MsftPartition, MsftPhysicalDisk, Win32DeviceGuard, Win32PhysicalMemory, Win32PnPSignedDriver,
    Win32Processor, Win32VideoController, attach_gpu_drivers, driver_info, media_by_letter,
    memory_integrity_on, memory_module,
};
use crate::{ScanStep, ScanWarning};

pub const CIMV2: &str = r"ROOT\CIMV2";
pub const STORAGE: &str = r"ROOT\Microsoft\Windows\Storage";
pub const DEVICE_GUARD: &str = r"ROOT\Microsoft\Windows\DeviceGuard";

/// Runs `SELECT … FROM <T's WMI class>` in `namespace`. Any failure becomes
/// one warning for `step` and an empty result.
pub fn query_in<T: DeserializeOwned>(
    namespace: &str,
    warnings: &mut Vec<ScanWarning>,
    step: ScanStep,
) -> Vec<T> {
    match WMIConnection::with_namespace_path(namespace).and_then(|c| c.query::<T>()) {
        Ok(rows) => rows,
        Err(e) => {
            let message = format!("WMI query in {namespace} failed: {e}");
            tracing::warn!("{message}");
            warnings.push(ScanWarning::new(step, message));
            Vec::new()
        }
    }
}

/// CPU name, cores and base clock from `Win32_Processor`.
pub fn collect_cpu(snap: &mut SystemSnapshot, warnings: &mut Vec<ScanWarning>) {
    let rows: Vec<Win32Processor> = query_in(CIMV2, warnings, ScanStep::Hardware);
    let Some(cpu) = rows.into_iter().next() else {
        return;
    };
    if let Some(name) = cpu
        .name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
    {
        snap.cpu.name = name;
    }
    if let Some(cores) = cpu.number_of_cores.filter(|&c| c > 0) {
        snap.cpu.physical_cores = cores;
    }
    if let Some(threads) = cpu.number_of_logical_processors.filter(|&t| t > 0) {
        snap.cpu.logical_cores = threads;
    }
    if let Some(mhz) = cpu.max_clock_speed.filter(|&m| m > 0) {
        snap.cpu.base_mhz = mhz;
    }
}

/// Driver version and date for the GPUs DXGI found.
pub fn collect_gpu_drivers(snap: &mut SystemSnapshot, warnings: &mut Vec<ScanWarning>) {
    let rows: Vec<Win32VideoController> = query_in(CIMV2, warnings, ScanStep::Hardware);
    attach_gpu_drivers(&mut snap.gpus, &rows);
}

pub fn collect_memory_modules(snap: &mut SystemSnapshot, warnings: &mut Vec<ScanWarning>) {
    let rows: Vec<Win32PhysicalMemory> = query_in(CIMV2, warnings, ScanStep::Memory);
    snap.memory.modules = rows.iter().map(memory_module).collect();
}

/// NVMe / SSD / HDD for each disk already in the snapshot, by drive letter.
pub fn collect_disk_media(snap: &mut SystemSnapshot, warnings: &mut Vec<ScanWarning>) {
    let disks: Vec<MsftPhysicalDisk> = query_in(STORAGE, warnings, ScanStep::Storage);
    let parts: Vec<MsftPartition> = query_in(STORAGE, warnings, ScanStep::Storage);
    let media = media_by_letter(&disks, &parts);
    for disk in &mut snap.disks {
        let letter = disk.mount.chars().next().map(|c| c.to_ascii_uppercase());
        if let Some(m) = letter.and_then(|l| media.get(&l)) {
            disk.media = *m;
        }
    }
}

pub fn collect_drivers(snap: &mut SystemSnapshot, warnings: &mut Vec<ScanWarning>) {
    let rows: Vec<Win32PnPSignedDriver> = query_in(CIMV2, warnings, ScanStep::Drivers);
    snap.drivers = rows.iter().filter_map(driver_info).collect();
}

pub fn collect_memory_integrity(snap: &mut SystemSnapshot, warnings: &mut Vec<ScanWarning>) {
    let rows: Vec<Win32DeviceGuard> = query_in(DEVICE_GUARD, warnings, ScanStep::OsSettings);
    if let Some(row) = rows.first() {
        snap.os.memory_integrity = memory_integrity_on(row);
    }
}

/// Every WMI collector, in scan order.
pub fn collect_wmi(snap: &mut SystemSnapshot, warnings: &mut Vec<ScanWarning>) {
    collect_cpu(snap, warnings);
    collect_gpu_drivers(snap, warnings);
    collect_memory_modules(snap, warnings);
    collect_disk_media(snap, warnings);
    collect_memory_integrity(snap, warnings);
    collect_drivers(snap, warnings);
}
