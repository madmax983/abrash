use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::Texture;
use abrash::tile_renderer::TileRenderer;
use abrash::zbuffer::ZBuffer;

#[test]
fn render_batch_textured_basic() {
    let width = 100;
    let height = 100;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);

    // Create a simple checkerboard texture
    let texture = Texture::checkered(32, 32, 0xFFFFFFFF, 0xFF000000).unwrap();

    // A single textured triangle
    let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
    let uv0 = Vec2::new(0.5, 0.0);
    let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
    let uv1 = Vec2::new(0.0, 1.0);
    let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
    let uv2 = Vec2::new(1.0, 1.0);

    let triangles = vec![(v0, uv0, v1, uv1, v2, uv2)];

    renderer.render_batch_textured(&mut fb, &mut zb, &triangles, &texture);
}
