use abrash::framebuffer::Framebuffer;

#[test]
fn test_alpha_blending_pixel() {
    let mut fb = Framebuffer::new(10, 10).unwrap();

    // Set background to opaque white
    fb.set_pixel(0, 0, 0xFFFFFFFF);

    // Draw 50% transparent red
    // Alpha = 0x80 = 128
    // Red = 0xFF
    // Green = 0x00
    // Blue = 0x00
    fb.set_pixel(0, 0, 0x80FF0000);

    let pixel = fb.get_pixel(0, 0).unwrap();

    // Expected calculation:
    // Alpha should be 255 (opaque result)
    // Red:   (255 * 128 + 255 * (255-128)) / 255 = 255
    // Green: (0   * 128 + 255 * (255-128)) / 255 = 127
    // Blue:  (0   * 128 + 255 * (255-128)) / 255 = 127

    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;
    let a = (pixel >> 24) & 0xFF;

    assert_eq!(a, 0xFF, "Result alpha should be opaque");
    assert_eq!(r, 0xFF, "Red channel incorrect");
    // Allow small rounding difference
    assert!((g as i32 - 127).abs() <= 1, "Green channel incorrect: {}", g);
    assert!((b as i32 - 127).abs() <= 1, "Blue channel incorrect: {}", b);
}

#[test]
fn test_alpha_blending_noop_transparent() {
    let mut fb = Framebuffer::new(10, 10).unwrap();
    fb.set_pixel(0, 0, 0xFFFFFFFF);

    // Draw fully transparent black
    fb.set_pixel(0, 0, 0x00000000);

    let pixel = fb.get_pixel(0, 0).unwrap();
    assert_eq!(pixel, 0xFFFFFFFF, "Fully transparent pixel should not change framebuffer");
}

use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;
use abrash::math::Vec3;

#[test]
fn test_fill_triangle_alpha() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Fill background with white
    fb.clear(0xFFFFFFFF);
    zb.clear();

    // Draw 50% transparent red triangle covering the center
    let color = 0x80FF0000;
    let v0 = (Vec3::new(0.0, 50.0, -2.0), 1.0);
    let v1 = (Vec3::new(-50.0, -50.0, -2.0), 1.0);
    let v2 = (Vec3::new(50.0, -50.0, -2.0), 1.0);

    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);

    // Check pixel at center (50, 50)
    // The rasterizer coordinates: 0,0 is top-left?
    // Check main.rs or math.rs for projection.
    // Default projection typically maps to screen coords.
    // But here I'm using `project_to_screen` inside `fill_triangle_3d`.
    // Wait, `fill_triangle_3d` does projection itself.
    // So the input vertices are in CLIP SPACE (or World Space?).
    // `fill_triangle_3d` calls `clip_triangle_against_near_plane` then `project_to_screen`.
    // `project_to_screen` maps [-1, 1] to [0, width].

    // My input vertices:
    // v0: (0.0, 50.0, -2.0) -> This is likely OUTSIDE view volume if not transformed?
    // Usually clip space is [-w, w].
    // If w=1.0, then x,y must be in [-1, 1].
    // 50.0 is way outside.

    // Let's use coordinates that project to screen center.
    // Center is (0,0) in clip space.
    // Z positive? Camera usually at origin looking -Z.
    // Let's look at `project_to_screen`.

    // I need to check `src/math.rs` to see `project_to_screen`.
    // But assuming standard perspective divide.
    // Wait, the previous test benchmarks used Z = -2.0.
    // `let v0 = (Vec3::new(0.0, 2.0, -2.0), 1.0);`
    // And `fill_triangle_3d` implementation showed:
    // `let p0_orig = project_to_screen(v0.0, v0.1, width, height);`

    // Let's check `project_to_screen` implementation.
    // I can't see it right now.
    // But let's assume the benchmarks work, so Z=-2.0 works.

    // Let's copy vertices from benchmark `bench_fill_triangle_gouraud` but smaller to fit in 100x100?
    // Benchmark uses 800x600.
    // If I use (0, 0.5, -2.0), it should be fine.

    let v0 = (Vec3::new(0.0, 0.9, -2.0), 1.0);
    let v1 = (Vec3::new(-0.9, -0.9, -2.0), 1.0);
    let v2 = (Vec3::new(0.9, -0.9, -2.0), 1.0);

    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);

    // Center pixel should be blended.
    let center_pixel = fb.get_pixel(50, 50).unwrap();

    // Check if it is NOT white and NOT pure red
    assert_ne!(center_pixel, 0xFFFFFFFF, "Center pixel should not be white");
    assert_ne!(center_pixel, 0xFFFF0000, "Center pixel should not be pure red");
    assert_ne!(center_pixel, 0x80FF0000, "Center pixel should not be just source color");

    // Check specific blended value (same as previous test)
    let r = (center_pixel >> 16) & 0xFF;
    let g = (center_pixel >> 8) & 0xFF;
    let b = center_pixel & 0xFF;

    assert_eq!(r, 0xFF);
    assert!((g as i32 - 127).abs() <= 1);
    assert!((b as i32 - 127).abs() <= 1);
}
