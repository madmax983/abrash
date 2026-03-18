//! Integration test: CpuRenderer produces pixel-identical output to direct TileRenderer usage.

use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::TileRenderer;
use abrash::render_api::Renderer;
use abrash::render_api::cpu_renderer::CpuRenderer;
use abrash::render_api::frame::{Frame, FrameCamera};
use abrash::render_api::material::Material;
use abrash::render_api::target::RenderTarget;
use abrash::zbuffer::ZBuffer;

const WIDTH: u32 = 200;
const HEIGHT: u32 = 200;

fn make_view_proj() -> (Mat4, Mat4) {
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 3.0),
        Vec3::ZERO,
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.57, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    (view, proj)
}

#[test]
fn test_cpu_renderer_matches_direct_tile_renderer() {
    let mesh = Mesh::cube(1.0);
    let color = 0xFFFF_0000u32;
    let transform = Mat4::identity();
    let (view, proj) = make_view_proj();
    let view_proj = view * proj;
    let mvp = transform * view_proj;

    // --- Direct TileRenderer path ---
    let mut fb_direct = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut zb_direct = ZBuffer::new(WIDTH, HEIGHT).unwrap();
    let mut tile_renderer = TileRenderer::new(WIDTH, HEIGHT);
    tile_renderer.enable_hiz(); // MUST match CpuRenderer which enables hiz by default

    fb_direct.clear(0xFF00_0000);
    zb_direct.clear();
    tile_renderer.begin_frame();

    let transformed: Vec<_> = mesh
        .vertices
        .iter()
        .map(|v| mvp.transform_point(*v))
        .collect();
    tile_renderer.submit_mesh(&mesh.indices, &transformed, color);
    tile_renderer.end_frame(&mut fb_direct, &mut zb_direct);

    // --- CpuRenderer path ---
    let mut renderer = CpuRenderer::new(WIDTH, HEIGHT);
    let mut target = RenderTarget::new(WIDTH, HEIGHT).unwrap();

    let mesh_h = renderer.create_mesh(&mesh).unwrap();
    let mat_h = renderer.create_material(Material::flat(color)).unwrap();

    let camera = FrameCamera::new(view, proj);
    let mut frame = Frame::new(camera);
    frame.draw(mesh_h, mat_h, transform);
    renderer.render_frame(&frame, &mut target).unwrap();

    // --- Compare pixel-by-pixel ---
    let direct_pixels = fb_direct.as_slice();
    let api_pixels = target.pixels();

    assert_eq!(
        direct_pixels.len(),
        api_pixels.len(),
        "buffer size mismatch"
    );

    let mut mismatches = 0usize;
    for (i, (&a, &b)) in direct_pixels.iter().zip(api_pixels.iter()).enumerate() {
        if a != b {
            mismatches += 1;
            if mismatches <= 5 {
                let x = i % WIDTH as usize;
                let y = i / WIDTH as usize;
                eprintln!("Pixel mismatch at ({x}, {y}): direct=0x{a:08X} api=0x{b:08X}");
            }
        }
    }

    assert_eq!(
        mismatches, 0,
        "CpuRenderer output differs from direct TileRenderer by {mismatches} pixels"
    );
}
