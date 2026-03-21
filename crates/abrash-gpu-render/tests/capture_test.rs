//! Integration tests for headless GPU capture.

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_gpu_render::capture::GpuCaptureTarget;
use abrash_gpu_render::renderer::GpuRenderer;
use abrash_render::render_api::frame::{Frame, FrameCamera};
use abrash_render::render_api::material::Material;

fn try_create_renderer() -> Option<GpuRenderer> {
    GpuRenderer::new_headless().ok()
}

fn contains_rgba(pixels: &[u8], rgba: [u8; 4]) -> bool {
    pixels.chunks_exact(4).any(|pixel| pixel == rgba)
}

/// Check if any pixel has a dominant channel matching the expected color.
/// Accounts for tone mapping + gamma changing exact values.
fn contains_dominant_color(pixels: &[u8], dominant_channel: usize) -> bool {
    pixels.chunks_exact(4).any(|pixel| {
        let v = pixel[dominant_channel];
        let others = (0..3)
            .filter(|&i| i != dominant_channel)
            .all(|i| pixel[i] < v / 2);
        v > 50 && others && pixel[3] > 200
    })
}

#[test]
fn test_capture_cube_has_visible_pixels() {
    let Some(mut renderer) = try_create_renderer() else {
        eprintln!("SKIP: no GPU adapter available");
        return;
    };

    let mesh = renderer.create_mesh(&Mesh::cube(1.0)).expect("create_mesh");
    let material = renderer.create_material(Material::flat(0xFFFF_4444));

    let camera = FrameCamera::new(
        Mat4::look_at(
            Vec3::new(0.0, 2.0, 5.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        ),
        Mat4::perspective(1.0, 320.0 / 240.0, 0.1, 100.0),
    );

    let mut frame = Frame::new(camera);
    frame.draw(mesh, material, Mat4::identity());

    let mut target = GpuCaptureTarget::new(renderer.device(), 320, 240);
    let capture = renderer.capture(&frame, &mut target).expect("capture");

    assert_eq!(capture.stats.width, 320);
    assert_eq!(capture.stats.height, 240);
    assert_eq!(capture.stats.batch_count, 1);
    assert_eq!(capture.stats.total_triangles, 12);
    assert!(capture.visible_pixel_count > 0);
    assert!(capture.coverage_percent() > 0.1);
    assert!(capture.to_compact_text().contains("COVERAGE"));
    assert_eq!(capture.pixels_rgba.len(), 320 * 240 * 4);
}

#[test]
fn test_capture_empty_scene_no_visible_pixels() {
    let Some(mut renderer) = try_create_renderer() else {
        eprintln!("SKIP: no GPU adapter available");
        return;
    };

    let camera = FrameCamera::new(
        Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        ),
        Mat4::perspective(1.0, 1.0, 0.1, 100.0),
    );

    let frame = Frame::new(camera);
    let mut target = GpuCaptureTarget::new(renderer.device(), 100, 100);
    let capture = renderer.capture(&frame, &mut target).expect("capture");

    assert_eq!(capture.visible_pixel_count, 0);
    assert_eq!(capture.stats.batch_count, 0);
    assert_eq!(capture.stats.total_triangles, 0);
}

#[test]
fn test_capture_two_cubes_preserves_both_material_colors() {
    let Some(mut renderer) = try_create_renderer() else {
        eprintln!("SKIP: no GPU adapter available");
        return;
    };

    let mesh = renderer.create_mesh(&Mesh::cube(1.0)).expect("create_mesh");
    let red = renderer.create_material(Material::flat(0xFFFF_0000));
    let green = renderer.create_material(Material::flat(0xFF00_FF00));

    let camera = FrameCamera::new(
        Mat4::look_at(
            Vec3::new(0.0, 2.0, 8.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        ),
        Mat4::perspective(1.0, 320.0 / 240.0, 0.1, 100.0),
    );

    let mut frame = Frame::new(camera);
    frame.draw(mesh, red, Mat4::translation(-2.0, 0.0, 0.0));
    frame.draw(mesh, green, Mat4::translation(2.0, 0.0, 0.0));

    let mut target = GpuCaptureTarget::new(renderer.device(), 320, 240);
    let capture = renderer.capture(&frame, &mut target).expect("capture");

    assert_eq!(capture.stats.batch_count, 2);
    assert_eq!(capture.stats.total_triangles, 24);
    assert!(capture.visible_pixel_count > 0);
    assert!(capture.to_compact_text().contains("2 batches"));
    // After tone mapping + gamma, exact 0xFF values become ~0xBA.
    // Check for dominant red/green channels instead of exact values.
    assert!(
        contains_dominant_color(&capture.pixels_rgba, 0),
        "should contain red-dominant pixels"
    );
    assert!(
        contains_dominant_color(&capture.pixels_rgba, 1),
        "should contain green-dominant pixels"
    );
}
