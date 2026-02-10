#![cfg(feature = "gpu-render")]

#[test]
fn gpu_render_crate_is_available_and_reexported() {
    let _ = std::any::type_name::<abrash::gpu_render::GpuTriangle>();
    let _ = std::any::type_name::<abrash_gpu_render::GpuTriangle>();
}
