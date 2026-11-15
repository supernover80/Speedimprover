//! GPU list from DXGI: real adapters only, accurate dedicated VRAM (WMI's
//! `AdapterRAM` caps at 4 GB), and integrated-vs-discrete from D3D12.

use std::ffi::c_void;

use si_core::model::GpuInfo;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0;
use windows::Win32::Graphics::Direct3D12::{
    D3D12_FEATURE_ARCHITECTURE, D3D12_FEATURE_DATA_ARCHITECTURE, D3D12CreateDevice, ID3D12Device,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_ERROR_NOT_FOUND, IDXGIAdapter1, IDXGIFactory1,
};

use crate::mapping::{integrated_by_name, is_skipped_adapter, vendor_from_pci_id};

const MIB: u64 = 1024 * 1024;

pub fn enumerate_gpus() -> windows::core::Result<Vec<GpuInfo>> {
    // SAFETY: plain COM factory creation; the returned interface is owned.
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1()? };
    let mut gpus = Vec::new();
    for index in 0.. {
        // SAFETY: valid factory; DXGI_ERROR_NOT_FOUND ends the enumeration.
        let adapter = match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) => adapter,
            Err(e) if e.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(e) => return Err(e),
        };
        // SAFETY: valid adapter interface.
        let desc = unsafe { adapter.GetDesc1()? };
        if is_skipped_adapter(desc.VendorId, desc.Flags) {
            continue;
        }
        let len = desc
            .Description
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(desc.Description.len());
        let name = String::from_utf16_lossy(&desc.Description[..len])
            .trim()
            .to_string();
        let is_integrated =
            shares_system_memory(&adapter).unwrap_or_else(|| integrated_by_name(&name));
        let luid = (u64::from(desc.AdapterLuid.HighPart as u32) << 32)
            | u64::from(desc.AdapterLuid.LowPart);
        gpus.push(GpuInfo {
            luid: Some(luid),
            vendor: vendor_from_pci_id(desc.VendorId),
            vram_mb: desc.DedicatedVideoMemory as u64 / MIB,
            is_integrated,
            name,
            ..Default::default()
        });
    }
    Ok(gpus)
}

/// `Some(true)` for unified-memory (integrated) GPUs; `None` if D3D12 is unavailable.
fn shares_system_memory(adapter: &IDXGIAdapter1) -> Option<bool> {
    let mut device: Option<ID3D12Device> = None;
    // SAFETY: valid adapter; on success `device` receives an owned interface.
    unsafe { D3D12CreateDevice(adapter, D3D_FEATURE_LEVEL_11_0, &mut device) }.ok()?;
    let device = device?;
    let mut arch = D3D12_FEATURE_DATA_ARCHITECTURE::default();
    // SAFETY: `arch` is the struct this feature expects, with its exact size.
    unsafe {
        device.CheckFeatureSupport(
            D3D12_FEATURE_ARCHITECTURE,
            &mut arch as *mut _ as *mut c_void,
            size_of::<D3D12_FEATURE_DATA_ARCHITECTURE>() as u32,
        )
    }
    .ok()?;
    Some(arch.UMA.as_bool())
}
