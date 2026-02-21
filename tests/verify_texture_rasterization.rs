use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::{FilterMode, Texture};
use abrash::zbuffer::ZBuffer;

#[test]
fn verify_texture_rasterization_output() {
    let width = 10;
    let height = 10;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Create a 2x2 texture
    // R G
    // B W
    let mut texture = Texture::new(2, 2).unwrap();
    let c_red = 0xFFFF_0000;
    let c_green = 0xFF00_FF00;
    let c_blue = 0xFF00_00FF;
    let c_white = 0xFFFF_FFFF;

    texture.set_pixel(0, 0, c_red);
    texture.set_pixel(1, 0, c_green);
    texture.set_pixel(0, 1, c_blue);
    texture.set_pixel(1, 1, c_white);

    // Vertex setup for a quad filling the screen (or part of it)
    // Screen is 10x10.
    // Let's draw a single triangle covering top-left to bottom-right.

    // v0: Top-Left (0,0) -> UV (0,0) (Red)
    // v1: Bottom-Left (0,10) -> UV (0,1) (Blue)
    // v2: Top-Right (10,0) -> UV (1,0) (Green)

    // Using Clip Space coordinates.
    // Visible range is [-w, w]. Let's say w=1.
    // Top-Left: (-1, 1)
    // Bottom-Left: (-1, -1)
    // Top-Right: (1, 1)
    // Winding: TL -> BL -> TR is CCW (Front Facing)

    let v0 = ((Vec3::new(-1.0, 1.0, 0.0), 1.0), Vec2::new(0.0, 0.0));
    let v1 = ((Vec3::new(-1.0, -1.0, 0.0), 1.0), Vec2::new(0.0, 1.0));
    let v2 = ((Vec3::new(1.0, 1.0, 0.0), 1.0), Vec2::new(1.0, 0.0));

    // Test Nearest Neighbor
    texture.filter_mode = FilterMode::Nearest;
    fb.clear(0);
    zb.clear();
    fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &texture);

    // Check pixel at (2, 2) (Top-Left quadrant) -> Should be Red
    // Check pixel at (8, 2) (Top-Right quadrant) -> Should be Green
    // Check pixel at (2, 8) (Bottom-Left quadrant) -> Should be Blue

    // Note: Rasterizer coordinates
    // (0,0) is top-left in framebuffer.
    // v0 corresponds to (-1, 1) in clip space, which maps to (0,0) screen space?
    // Project triangle to screen:
    // x = (x/w + 1) * w/2
    // y = (1 - y/w) * h/2
    // (-1, 1) -> x=0, y=0. Correct.
    // (1, 1) -> x=10, y=0. Correct.
    // (-1, -1) -> x=0, y=10. Correct.

    let p_tl = fb.get_pixel(2, 2);
    assert_eq!(p_tl, Some(c_red), "Nearest: TL pixel should be Red");

    let p_tr = fb.get_pixel(7, 2);
    // It interpolates. UV at x=7 is 0.7. Round to nearest?
    // Nearest neighbor at 0.7 is index 1 (since 2 * 0.7 = 1.4 -> 1).
    assert_eq!(p_tr, Some(c_green), "Nearest: TR pixel should be Green");

    let p_bl = fb.get_pixel(2, 7);
    assert_eq!(p_bl, Some(c_blue), "Nearest: BL pixel should be Blue");

    // Test Bilinear
    texture.filter_mode = FilterMode::Bilinear;
    fb.clear(0);
    zb.clear();
    fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &texture);

    // Center pixel (5, 5) should be average of all 4?
    // Actually triangle is top-left half. (5,5) is on the edge 0->1? No.
    // Triangle is (0,0)-(10,0)-(0,10).
    // (5,5) is exactly on the diagonal edge.
    // UV at (5,5)?
    // Barycentric coords: p = u*v0 + v*v1 + w*v2
    // (5,5) = 0.0*v0 + 0.5*v1 + 0.5*v2
    // v0=(0,0), v1=(10,0), v2=(0,10)
    // 0.5*(10,0) + 0.5*(0,10) = (5, 5). Correct.
    // UV = 0.5*UV1 + 0.5*UV2 = 0.5*(1,0) + 0.5*(0,1) = (0.5, 0.5).

    // Bilinear sample at (0.5, 0.5) of 2x2 texture.
    // Should be blend of all 4.

    let p_center = fb.get_pixel(4, 4); // Slightly inside to avoid edge precision issues
    // UV at (4,4) ->
    // 4 = alpha * 0 + beta * 10 + gamma * 0 -> 10*beta = 4 -> beta = 0.4
    // 4 = alpha * 0 + beta * 0 + gamma * 10 -> 10*gamma = 4 -> gamma = 0.4
    // alpha = 1 - 0.4 - 0.4 = 0.2
    // UV = 0.2*(0,0) + 0.4*(1,0) + 0.4*(0,1) = (0.4, 0.4)

    // Bilinear sample at (0.4, 0.4)
    // It's close to center.
    // I won't assert exact value, just that it's not black/empty.
    assert_ne!(p_center, Some(0), "Bilinear: Center pixel should be drawn");
    assert_ne!(
        p_center,
        Some(c_red),
        "Bilinear: Center pixel should be blended (not pure red)"
    );
}
