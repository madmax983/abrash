use abrash::framebuffer::Framebuffer;
use abrash::math::Vec2;
use abrash::math::Vec3;
use abrash::rasterizer::Texture;
use abrash::tile_renderer::TileRenderer;
use abrash::zbuffer::ZBuffer;

#[test]
fn textured_triangle_renders_correctly() {
    let width = 100;
    let height = 100;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);

    // Create a 2x2 texture with distinctive colors
    // (0,0): Red (FF0000)
    // (1,0): Green (00FF00)
    // (0,1): Blue (0000FF)
    // (1,1): Yellow (FFFF00)
    let mut texture = Texture::new(2, 2).unwrap();
    texture.set_pixel(0, 0, 0xFFFF0000);
    texture.set_pixel(1, 0, 0xFF00FF00);
    texture.set_pixel(0, 1, 0xFF0000FF);
    texture.set_pixel(1, 1, 0xFFFFFF00);

    // Define a triangle covering the screen with UVs mapping to texture corners
    // v0: Top-left (-1, 1) -> Screen (0,0) -> UV(0,0) -> Red
    // v1: Bottom-left (-1, -1) -> Screen (0,100) -> UV(0,1) -> Blue
    // v2: Bottom-right (1, -1) -> Screen (100,100) -> UV(1,1) -> Yellow
    // Note: Z=5.0 for all vertices (flat depth)
    let v0 = ((Vec3::new(-1.0, 1.0, 5.0), 1.0), Vec2::new(0.0, 0.0));
    let v1 = ((Vec3::new(-1.0, -1.0, 5.0), 1.0), Vec2::new(0.0, 1.0));
    let v2 = ((Vec3::new(1.0, -1.0, 5.0), 1.0), Vec2::new(1.0, 1.0));

    // This method doesn't exist yet, so compilation should fail
    renderer.render_batch_textured(&mut fb, &mut zb, &texture, &[(v0, v1, v2)]);

    // Check a pixel that should be Red (near top-left)
    // (10, 10) corresponds to UV ~ (0.1, 0.1) -> Texel (0,0) -> Red
    let pixel = fb.get_pixel(10, 10).unwrap();
    assert_eq!(pixel, 0xFFFF0000, "Pixel at (10,10) should be Red");

    // Check a pixel that should be Blue (near bottom-left)
    // (10, 90) corresponds to UV ~ (0.1, 0.9) -> Texel (0,1) -> Blue
    let pixel = fb.get_pixel(10, 90).unwrap();
    assert_eq!(pixel, 0xFF0000FF, "Pixel at (10,90) should be Blue");

    // Check a pixel that should be Yellow (near bottom-right)
    // (90, 90) corresponds to UV ~ (0.9, 0.9) -> Texel (1,1) -> Yellow
    let pixel = fb.get_pixel(90, 90).unwrap();
    assert_eq!(pixel, 0xFFFFFF00, "Pixel at (90,90) should be Yellow");
}
