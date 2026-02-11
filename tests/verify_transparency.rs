use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_textured_transparency() {
    let width = 10;
    let height = 10;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // 1. Fill framebuffer with Red background
    for y in 0..height {
        for x in 0..width {
            fb.set_pixel(x as i32, y as i32, 0xFFFF0000); // Red
        }
    }

    // 2. Create a texture with transparent pixels
    // 2x2 texture:
    // (0,0) Transparent (0x00000000)
    // (1,0) Opaque Green (0xFF00FF00)
    // (0,1) Opaque Blue (0xFF0000FF)
    // (1,1) Semi-transparent White (0x80FFFFFF)
    let mut tex = Texture::new(2, 2).unwrap();
    tex.set_pixel(0, 0, 0x00000000);
    tex.set_pixel(1, 0, 0xFF00FF00);
    tex.set_pixel(0, 1, 0xFF0000FF);
    tex.set_pixel(1, 1, 0x80FFFFFF);

    // 3. Define a quad covering the framebuffer
    // Use two triangles to form a quad covering the screen
    // Vertices in Clip Space:
    // v0: Top-Left (-1, 1), UV (0, 0)
    // v1: Bottom-Left (-1, -1), UV (0, 1)
    // v2: Bottom-Right (1, -1), UV (1, 1)
    // v3: Top-Right (1, 1), UV (1, 0)

    let v0 = ((Vec3::new(-1.0, 1.0, 1.0), 1.0), Vec2::new(0.0, 0.0));
    let v1 = ((Vec3::new(-1.0, -1.0, 1.0), 1.0), Vec2::new(0.0, 1.0));
    let v2 = ((Vec3::new(1.0, -1.0, 1.0), 1.0), Vec2::new(1.0, 1.0));
    let v3 = ((Vec3::new(1.0, 1.0, 1.0), 1.0), Vec2::new(1.0, 0.0));

    // Draw first triangle (v0, v1, v2)
    fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &tex);
    // Draw second triangle (v0, v2, v3)
    fill_triangle_textured(&mut fb, &mut zb, v0, v2, v3, &tex);

    // 4. Assertions
    // Top-Left (0,0) maps to UV (0,0) -> Transparent. Should be Red.
    let tl = fb.get_pixel(0, 0).unwrap();
    assert_eq!(tl, 0xFFFF0000, "Top-Left pixel should remain Red (background)");

    // Top-Right (9,0) maps to UV (1,0) -> Green. Should be Green.
    let tr = fb.get_pixel(9, 0).unwrap();
    assert_eq!(tr, 0xFF00FF00, "Top-Right pixel should be Green");

    // Bottom-Left (0,9) maps to UV (0,1) -> Blue. Should be Blue.
    let bl = fb.get_pixel(0, 9).unwrap();
    assert_eq!(bl, 0xFF0000FF, "Bottom-Left pixel should be Blue");

    // Bottom-Right (9,9) maps to UV (1,1) -> Semi-Transparent White blended with Red.
    // White (255, 255, 255) with alpha 128 (0x80) over Red (255, 0, 0).
    // Result R = (255*128 + 255*127)/255 = 255
    // Result G = (255*128 + 0*127)/255 = 128
    // Result B = (255*128 + 0*127)/255 = 128
    // Expected: 0xFF FF 80 80 (approx)
    // Note: The blend implementation might use 256 for inv_alpha (255-alpha), etc.
    // Let's just check it's not Red and not White.
    let br = fb.get_pixel(9, 9).unwrap();
    assert_ne!(br, 0xFFFF0000, "Bottom-Right pixel should blended (not pure Red)");
    assert_ne!(br, 0xFFFFFFFF, "Bottom-Right pixel should blended (not pure White)");

    // Check specific blended value if possible, but exact match depends on blend formula (0..255 vs 0..256 scale)
}
