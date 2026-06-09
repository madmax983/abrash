use abrash_gpu_render::raytracing::RtShadowPass;
use abrash_gpu_render::device::{GpuDeviceConfig, GpuDevice};

#[test]
fn test_rt_shadow_encode_panics_without_ensure_output() {
    let config = GpuDeviceConfig::headless();
    let gpu = match GpuDevice::new_headless(&config) {
        Ok(gpu) => gpu,
        Err(_) => return, // Skip test if no GPU available
    };

    // Skip if RT features are not supported
    if !gpu.device().features().contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY) {
        return;
    }

    let device = gpu.device();
    let queue = gpu.queue();

    let shadow_pass = RtShadowPass::new(device);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("test"),
    });

    let dummy_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let dummy_view = dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let instances: Vec<(&abrash_gpu_render::accel_structure::MeshBlas, &abrash_core::math::Mat4)> = vec![];
    let tlas = abrash_gpu_render::accel_structure::SceneTlas::build(device, queue, &instances);

    // We use catch_unwind to assert the panic happens, but only after we are sure we've reached this point.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        shadow_pass.encode(device, queue, &mut encoder, &dummy_view, &tlas, [0.0, -1.0, 0.0]);
    }));

    assert!(result.is_err(), "Expected encode to panic without ensure_output");

    let err = result.unwrap_err();
    let panic_msg = if let Some(s) = err.downcast_ref::<&str>() {
        *s
    } else if let Some(s) = err.downcast_ref::<String>() {
        s.as_str()
    } else {
        "Unknown panic type"
    };

    assert!(
        panic_msg.contains("call ensure_output first"),
        "Expected panic message to contain 'call ensure_output first', got: {}",
        panic_msg
    );
}
