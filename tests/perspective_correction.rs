use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::{Texture, fill_triangle_textured};
use abrash::zbuffer::ZBuffer;

#[test]
fn test_perspective_distortion() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Create a vertical gradient texture: 0 at top (v=0), 255 at bottom (v=1)
    let mut tex = Texture::new(32, 32).unwrap();
    for y in 0..32 {
        let val = (y as f32 / 31.0 * 255.0) as u32;
        let color = 0xFF000000 | (val << 16) | (val << 8) | val;
        for x in 0..32 {
            tex.set_pixel(x, y, color);
        }
    }

    // Define a triangle that leans back significantly.
    // Using coordinate system where Y is Up.
    // Top Edge (y=1) at z_near (w=1).
    // Bottom Vertex (y=-1) at z_far (w=4).

    // V0: Top-Left
    let v0_pos = Vec3::new(-1.0, 1.0, -1.0);
    let v0_w = 1.0;
    let v0_uv = Vec2::new(0.0, 0.0);

    // V1: Top-Right
    let v1_pos = Vec3::new(1.0, 1.0, -1.0);
    let v1_w = 1.0;
    let v1_uv = Vec2::new(1.0, 0.0);

    // V2: Bottom-Center
    let v2_pos = Vec3::new(0.0, -1.0, -4.0); // Z doesn't matter much for projection if W is provided, but good for consistency
    let v2_w = 4.0;
    let v2_uv = Vec2::new(0.5, 1.0);

    // Project Logic Check (mental):
    // Top Y_screen_ndc = 1.0 / 1.0 = 1.0.
    // Bottom Y_screen_ndc = -1.0 / 4.0 = -0.25.
    // Screen Y maps (1.0) -> 0 and (-0.25) -> Height * (1 - (-0.25))/2 = Height * 0.625.
    //
    // Wait, project_to_screen logic:
    // screen_y = ((1.0 - ndc_y) * 0.5 * height)
    //
    // Top (ndc=1.0): (1-1)*... = 0.
    // Bottom (ndc=-0.25): (1 - (-0.25)) * 0.5 * 100 = 1.25 * 50 = 62.5 -> 62.
    //
    // Midpoint Y in pixels = (0 + 62) / 2 = 31.
    //
    // At Pixel Y=31:
    // Screen Y fraction = 31/100 = 0.31.
    // NDC Y back calculation:
    // 31 = (1 - ndc) * 50 => 31/50 = 1 - ndc => 0.62 = 1 - ndc => ndc = 0.38.
    //
    // Solve for t (barycentric coord from top to bottom) such that projected y(t) = 0.38.
    // y_world(t) = 1 * (1-t) + (-1) * t = 1 - 2t
    // w_world(t) = 1 * (1-t) + 4 * t = 1 + 3t
    // ndc(t) = y_world / w_world = (1 - 2t) / (1 + 3t)
    //
    // 0.38 = (1 - 2t) / (1 + 3t)
    // 0.38(1 + 3t) = 1 - 2t
    // 0.38 + 1.14t = 1 - 2t
    // 3.14t = 0.62
    // t = 0.62 / 3.14 ≈ 0.197 (~0.2).
    //
    // So correct V coordinate should be ~0.2.
    //
    // Affine interpolation logic (current implementation):
    // It interpolates screen space Y linearly.
    // At pixel 31 (halfway between 0 and 62), it considers it halfway through the triangle in screen space.
    // So it yields V ~ 0.5.
    //
    // Expected Pixel Value (Perspective): 0.2 * 255 ≈ 51.
    // Actual Pixel Value (Affine): 0.5 * 255 ≈ 127.
    //
    // We will assert that the value is < 80 (allowing some margin, but well below 127).

    fill_triangle_textured(
        &mut fb,
        &mut zb,
        ((v0_pos, v0_w), v0_uv),
        ((v1_pos, v1_w), v1_uv),
        ((v2_pos, v2_w), v2_uv),
        &tex,
    );

    let pixel_y = 31;
    let pixel_x = 50; // Center X

    let pixel_color = fb.get_pixel(pixel_x, pixel_y).unwrap();
    let blue_channel = pixel_color & 0xFF;

    println!("Pixel at (50, 31) Blue channel: {}", blue_channel);

    // With Affine mapping, this will fail.
    // We expect perspective correction to give us a value closer to 51.
    assert!(
        blue_channel < 80,
        "Pixel value {} is too high, indicating affine mapping (expected < 80, got ~127)",
        blue_channel
    );
}
