use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_gpu_render::capture::GpuCaptureTarget;
use abrash_gpu_render::renderer::GpuRenderer;
use abrash_render::render_api::frame::{Frame, FrameCamera};

fn try_create_renderer() -> Option<GpuRenderer> {
    GpuRenderer::new_headless().ok()
}

#[test]
fn should_panic_if_capture_called_without_valid_frame() {
    let Some(mut renderer) = try_create_renderer() else {
        return;
    };
    // Need to test missing GBuffer?
}
