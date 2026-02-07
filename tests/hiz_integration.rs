/// Integration tests for Hi-Z buffer with tile renderer
///
/// These tests verify that:
/// 1. `TileRenderer` with Hi-Z produces identical output to without Hi-Z
/// 2. Hi-Z actually culls occluded triangles
/// 3. Occlusion queries are correct (no false negatives)
use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::tile_renderer::{ClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;

/// Verify that rendering with Hi-Z produces pixel-identical output to without Hi-Z
#[test]
fn hiz_produces_identical_output() {
    let width = 800;
    let height = 600;

    // Create two scenes: one with Hi-Z, one without
    let mut fb_with_hiz = Framebuffer::new(width, height).unwrap();
    let mut zb_with_hiz = ZBuffer::new(width, height).unwrap();
    let mut renderer_with_hiz = TileRenderer::new(width, height);
    renderer_with_hiz.enable_hiz();

    let mut fb_without_hiz = Framebuffer::new(width, height).unwrap();
    let mut zb_without_hiz = ZBuffer::new(width, height).unwrap();
    let mut renderer_without_hiz = TileRenderer::new(width, height);

    // Test scene: 3 triangles with some overlap
    let triangles: Vec<ClipTriangle> = vec![
        // Triangle 1: Front, red
        (
            (Vec3::new(-0.5, -0.5, 0.3), 1.0),
            (Vec3::new(0.5, -0.5, 0.3), 1.0),
            (Vec3::new(0.0, 0.5, 0.3), 1.0),
            0xFF0000FF,
        ),
        // Triangle 2: Back, green (should be occluded by triangle 1 in center)
        (
            (Vec3::new(-0.3, -0.3, 0.7), 1.0),
            (Vec3::new(0.3, -0.3, 0.7), 1.0),
            (Vec3::new(0.0, 0.3, 0.7), 1.0),
            0x00FF00FF,
        ),
        // Triangle 3: Side, blue (partially visible)
        (
            (Vec3::new(0.3, 0.0, 0.5), 1.0),
            (Vec3::new(0.9, 0.0, 0.5), 1.0),
            (Vec3::new(0.6, 0.6, 0.5), 1.0),
            0x0000FFFF,
        ),
    ];

    // Render both versions
    renderer_with_hiz.render_batch(&mut fb_with_hiz, &mut zb_with_hiz, &triangles);
    renderer_without_hiz.render_batch(&mut fb_without_hiz, &mut zb_without_hiz, &triangles);

    // Compare framebuffers pixel-by-pixel
    let pixels_with_hiz = fb_with_hiz.as_slice();
    let pixels_without_hiz = fb_without_hiz.as_slice();

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            assert_eq!(
                pixels_with_hiz[idx], pixels_without_hiz[idx],
                "Pixel mismatch at ({}, {}): with Hi-Z = 0x{:08X}, without Hi-Z = 0x{:08X}",
                x, y, pixels_with_hiz[idx], pixels_without_hiz[idx]
            );
        }
    }

    // Compare zbuffers
    let depths_with_hiz = zb_with_hiz.as_slice();
    let depths_without_hiz = zb_without_hiz.as_slice();

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let d1 = depths_with_hiz[idx];
            let d2 = depths_without_hiz[idx];

            // Handle infinities specially
            if d1.is_infinite() && d2.is_infinite() {
                continue; // Both infinite, considered equal
            }

            assert!(
                (d1 - d2).abs() < 0.0001,
                "Depth mismatch at ({x}, {y}): with Hi-Z = {d1}, without Hi-Z = {d2}"
            );
        }
    }
}

/// Verify that Hi-Z doesn't introduce false negatives (skipping visible triangles)
#[test]
fn hiz_no_false_negatives() {
    let width = 1920;
    let height = 1080;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);
    renderer.enable_hiz();

    // Test scene: Single triangle should always be visible
    let triangles: Vec<ClipTriangle> = vec![(
        (Vec3::new(-0.8, -0.8, 0.5), 1.0),
        (Vec3::new(0.8, -0.8, 0.5), 1.0),
        (Vec3::new(0.0, 0.8, 0.5), 1.0),
        0xFF0000FF, // Red
    )];

    renderer.render_batch(&mut fb, &mut zb, &triangles);

    // Count non-background pixels
    let mut drawn_pixels = 0;
    for &pixel in fb.as_slice() {
        if pixel != 0xFF000000 {
            // Not background
            drawn_pixels += 1;
        }
    }

    // Triangle should have drawn something
    assert!(
        drawn_pixels > 0,
        "Hi-Z incorrectly culled a visible triangle (drew {drawn_pixels} pixels)"
    );

    // Verify specific center pixel is red
    let center_x = width / 2;
    let center_y = height / 2;
    let center_pixel = fb.as_slice()[(center_y * width + center_x) as usize];
    assert_eq!(
        center_pixel & 0x00FFFFFF,
        0x000000FF, // Blue component only (BGRA format)
        "Center pixel should be red, got 0x{center_pixel:08X}"
    );
}

/// Verify that Hi-Z culls occluded geometry in multi-frame scenario
#[test]
fn hiz_culls_occluded_geometry_across_frames() {
    let width = 800;
    let height = 600;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);
    renderer.enable_hiz();

    // Frame 1: Render a front triangle
    let front_triangles: Vec<ClipTriangle> = vec![(
        (Vec3::new(-0.5, -0.5, 0.2), 1.0),
        (Vec3::new(0.5, -0.5, 0.2), 1.0),
        (Vec3::new(0.0, 0.5, 0.2), 1.0),
        0xFF0000FF, // Red (closer)
    )];

    renderer.render_batch(&mut fb, &mut zb, &front_triangles);

    // Frame 2: Render the same front triangle plus a back triangle
    // The Hi-Z pyramid from frame 1 should help cull the back triangle
    let both_triangles: Vec<ClipTriangle> = vec![
        (
            (Vec3::new(-0.5, -0.5, 0.2), 1.0),
            (Vec3::new(0.5, -0.5, 0.2), 1.0),
            (Vec3::new(0.0, 0.5, 0.2), 1.0),
            0xFF0000FF, // Red (closer)
        ),
        (
            (Vec3::new(-0.4, -0.4, 0.8), 1.0), // Back triangle
            (Vec3::new(0.4, -0.4, 0.8), 1.0),
            (Vec3::new(0.0, 0.4, 0.8), 1.0),
            0x00FF00FF, // Green (farther, should be culled)
        ),
    ];

    renderer.render_batch(&mut fb, &mut zb, &both_triangles);

    // Count green pixels (there should be very few or none in the overlap region)
    let mut green_pixels = 0;
    for y in height / 4..3 * height / 4 {
        for x in width / 4..3 * width / 4 {
            let pixel = fb.as_slice()[(y * width + x) as usize];
            // Check if pixel is predominantly green (G > R and G > B)
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;
            if g > r && g > b && g > 128 {
                green_pixels += 1;
            }
        }
    }

    // In the center overlap region, there should be no green pixels (fully occluded)
    assert_eq!(
        green_pixels, 0,
        "Found {green_pixels} green pixels in overlap region, expected 0 (occlusion failed)"
    );
}

/// Test Hi-Z with empty scene (should not crash or produce errors)
#[test]
fn hiz_handles_empty_scene() {
    let width = 800;
    let height = 600;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);
    renderer.enable_hiz();

    let triangles: Vec<ClipTriangle> = vec![];

    // Should not panic or error
    renderer.render_batch(&mut fb, &mut zb, &triangles);
}

/// Test Hi-Z with high triangle count (stress test)
#[test]
fn hiz_handles_high_triangle_count() {
    let width = 1920;
    let height = 1080;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);
    renderer.enable_hiz();

    // Generate 100 triangles at various depths
    let mut triangles = Vec::new();
    for i in 0..100 {
        let depth = 0.1 + (i as f32 * 0.008); // 0.1 to 0.9
        let offset_x = ((i % 10) as f32 - 5.0) * 0.15;
        let offset_y = ((i / 10) as f32 - 5.0) * 0.15;

        triangles.push((
            (Vec3::new(offset_x - 0.1, offset_y - 0.1, depth), 1.0),
            (Vec3::new(offset_x + 0.1, offset_y - 0.1, depth), 1.0),
            (Vec3::new(offset_x, offset_y + 0.1, depth), 1.0),
            0xFF000000 | ((i * 2) << 16) | ((i * 3) << 8) | (i * 5), // Unique color
        ));
    }

    // Should complete without panic
    renderer.render_batch(&mut fb, &mut zb, &triangles);

    // Verify something was drawn
    let mut drawn_pixels = 0;
    for &pixel in fb.as_slice() {
        if pixel != 0xFF000000 {
            drawn_pixels += 1;
        }
    }

    assert!(
        drawn_pixels > 0,
        "Hi-Z with 100 triangles drew nothing (should draw something)"
    );
}
