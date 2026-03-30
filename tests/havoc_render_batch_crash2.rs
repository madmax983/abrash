use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::{ClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;

#[test]
fn havoc_bin_two_level_panic() {
    let width = 256;
    let height = 256;
    let mut renderer = TileRenderer::new(width, height);
    renderer.enable_software_two_level_binning();

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let v0 = (Vec3::new(f32::NAN, 100.0, 1.0), 1.0);
    let v1 = (Vec3::new(-100.0, f32::NAN, 1.0), 1.0);
    let v2 = (Vec3::new(0.0, -100.0, f32::NAN), 1.0);

    let triangles = vec![(v0, v1, v2, 0xFFFF_FFFF)];

    renderer.render_batch(&mut fb, &mut zb, &triangles);
}
