use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::tile::TileRenderer;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;

#[test]
fn verify_textured_rendering_output() {
    let width = 64;
    let height = 64;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);

    // Create a simple 2x2 texture
    // (0,0)=Red, (1,0)=Green, (0,1)=Blue, (1,1)=White
    let mut texture = Texture::new(2, 2).unwrap();
    texture.set_pixel(0, 0, 0xFFFF0000);
    texture.set_pixel(1, 0, 0xFF00FF00);
    texture.set_pixel(0, 1, 0xFF0000FF);
    texture.set_pixel(1, 1, 0xFFFFFFFF);

    // Define a triangle covering the center
    // v0: Top-Left (Red)
    let v0 = (Vec3::new(-0.5, 0.5, 1.0), 1.0);
    let uv0 = Vec2::new(0.0, 0.0);

    // v1: Bottom-Left (Blue)
    let v1 = (Vec3::new(-0.5, -0.5, 1.0), 1.0);
    let uv1 = Vec2::new(0.0, 1.0);

    // v2: Bottom-Right (White)
    let v2 = (Vec3::new(0.5, -0.5, 1.0), 1.0);
    let uv2 = Vec2::new(1.0, 1.0);

    renderer.render_batch_textured(&mut fb, &mut zb, &[(v0, uv0, v1, uv1, v2, uv2)], &texture);

    // Check specific pixels

    // 1. Inside the triangle
    let p_in = fb.get_pixel(26, 37).unwrap();
    assert_ne!(
        p_in, 0xFF000000,
        "Pixel inside triangle should not be black background"
    );

    // 2. Outside the triangle
    // (0.5, 0.5) -> (48, 16) is definitely outside the triangle (top right quadrant)
    let p_out = fb.get_pixel(48, 16).unwrap();
    assert_eq!(
        p_out, 0xFF000000,
        "Pixel outside triangle should be black opaque background"
    );
}
