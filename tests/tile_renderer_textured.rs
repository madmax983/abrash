use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::texture::Texture;
use abrash::rasterizer::{TexturedClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;

#[test]
fn test_textured_triangle_rendering() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tr = TileRenderer::new(width, height);

    // Create a 2x2 texture
    // (0,0) Red, (1,0) Green
    // (0,1) Blue, (1,1) White
    let mut tex = Texture::new(2, 2).unwrap();
    tex.set_pixel(0, 0, 0xFFFF0000); // Red
    tex.set_pixel(1, 0, 0xFF00FF00); // Green
    tex.set_pixel(0, 1, 0xFF0000FF); // Blue
    tex.set_pixel(1, 1, 0xFFFFFFFF); // White

    // A single triangle covering the screen
    // Top-left (Red), Top-right (Green), Bottom-left (Blue)
    // Corresponds to UVs (0,0), (1,0), (0,1)
    // Vertices in clip space (z=5.0, w=5.0 means z=1.0 in NDC, or actually z/w=1.0)
    // Wait, clip space: x/w, y/w, z/w.
    // Screen coords: x in [-1, 1] -> [0, width]

    // V0: (0, 100) -> NDC (-1, 1)? No, (0,0) is top-left in screen space usually?
    // Project to screen uses:
    // screen_x = ((ndc_x + 1.0) * 0.5 * width)
    // screen_y = ((1.0 - ndc_y) * 0.5 * height)

    // Top-Left: Screen (0, 0) -> NDC (-1, 1)
    // Top-Right: Screen (100, 0) -> NDC (1, 1)
    // Bottom-Left: Screen (0, 100) -> NDC (-1, -1)

    // So vertices:
    let _v0 = (Vec3::new(-1.0, 1.0, 1.0), 1.0); // Top-Left
    let _uv0 = Vec2::new(0.0, 0.0);

    let _v1 = (Vec3::new(1.0, 1.0, 1.0), 1.0); // Top-Right (Wait, winding order?)
    // CCW winding is front face?
    // Top-Left -> Bottom-Left -> Top-Right?
    // (-1, 1) -> (-1, -1) -> (1, 1)
    // Let's check is_backface logic.
    // ux = p1.x - p0.x, uy = p1.y - p0.y
    // vx = p2.x - p0.x, vy = p2.y - p0.y
    // nz = ux*vy - uy*vx
    // If nz >= 0, it's backface (culled).
    // Screen Y increases downwards.

    // V0 (TL): (0, 0)
    // V1 (BL): (0, 100)
    // V2 (TR): (100, 0)

    // ux = 0 - 0 = 0
    // uy = 100 - 0 = 100
    // vx = 100 - 0 = 100
    // vy = 0 - 0 = 0
    // nz = 0*0 - 100*100 = -10000 < 0. Front face!

    // So order: TL, BL, TR.
    // V0: TL (-1, 1) UV (0, 0) -> Red
    // V1: BL (-1, -1) UV (0, 1) -> Blue
    // V2: TR (1, 1) UV (1, 0) -> Green

    let v0 = (Vec3::new(-1.0, 1.0, 1.0), 1.0);
    let uv0 = Vec2::new(0.0, 0.0);

    let v1 = (Vec3::new(-1.0, -1.0, 1.0), 1.0);
    let uv1 = Vec2::new(0.0, 1.0);

    let v2 = (Vec3::new(1.0, 1.0, 1.0), 1.0);
    let uv2 = Vec2::new(1.0, 0.0);

    let triangles: Vec<TexturedClipTriangle> = vec![(v0, uv0, v1, uv1, v2, uv2)];

    tr.render_batch_textured(&mut fb, &mut zb, &triangles, &tex);

    // Check pixels
    // (0,0) should be Red
    // (0, 99) should be Blue
    // (99, 0) should be Green

    // Note: Texture sampling at UV (0,0) might sample the pixel center or corner depending on implementation.
    // Nearest neighbor at (0,0) -> pixel (0,0) -> Red.

    let p_tl = fb.get_pixel(0, 0).unwrap();
    assert_eq!(p_tl, 0xFFFF0000, "Top-left should be red");

    let p_bl = fb.get_pixel(0, 99).unwrap();
    assert_eq!(p_bl, 0xFF0000FF, "Bottom-left should be blue");

    let p_tr = fb.get_pixel(99, 0).unwrap();
    assert_eq!(p_tr, 0xFF00FF00, "Top-right should be green");
}
