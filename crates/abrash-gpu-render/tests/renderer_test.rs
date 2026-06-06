use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_gpu_render::capture::GpuCaptureTarget;
use abrash_gpu_render::renderer::GpuRenderer;
use abrash_render::render_api::frame::{Frame, FrameCamera};
use abrash_render::render_api::material::Material;

fn try_create_renderer() -> Option<GpuRenderer> {
    if let Ok(renderer) = GpuRenderer::new_headless() {
        #[cfg(feature = "ray-tracing")]
        if !renderer
            .device()
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
        {
            return None;
        }
        Some(renderer)
    } else {
        None
    }
}

#[test]
fn test_renderer_gbuffer_uninit_panic() {
    let Some(mut renderer) = try_create_renderer() else {
        return;
    };
    // DO NOT call ensure_gbuffer, just test that we can trigger a panic and then we'll fix the code to return Result.

    // Actually we can't test private methods.
}
