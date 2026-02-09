#![cfg(feature = "gpu-binning")]

#[test]
fn gpu_crate_is_available_and_reexported() {
    let _ = std::any::type_name::<abrash::gpu::GpuBinner>();
    let _ = std::any::type_name::<abrash_gpu::GpuBinner>();
}
