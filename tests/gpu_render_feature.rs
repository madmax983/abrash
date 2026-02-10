#![cfg(feature = "gpu-render")]

#[test]
fn gpu_render_module_is_exposed() {
    let _ = std::any::type_name::<abrash::gpu_render::GpuTriangle>();
}
