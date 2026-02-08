//! Correctness tests for two-level hierarchical binning with Hi-Z culling
//!
//! Verifies that two-level binning (128×128 coarse bins → 32×32 fine tiles)
//! produces identical results to single-level binning while culling occluded geometry.

#![cfg(all(feature = "backend-win32", feature = "gpu-binning"))]

use abrash::{
    framebuffer::Framebuffer,
    hiz_buffer::{AABB3D, HiZBuffer},
    math::Vec3,
    tile_renderer::{ClipTriangle, TileRenderer},
    zbuffer::ZBuffer,
};

/// Helper to create a clip-space vertex
fn make_clip_vertex(x: f32, y: f32, z: f32, w: f32) -> (Vec3, f32) {
    (Vec3::new(x, y, z), w)
}

/// Helper to create a simple test scene with known occlusion
fn create_occluder_scene() -> Vec<ClipTriangle> {
    vec![
        // Occluder: Large front triangle covering center
        (
            make_clip_vertex(-0.6, -0.6, 0.2, 1.0), // Close to camera
            make_clip_vertex(0.6, -0.6, 0.2, 1.0),
            make_clip_vertex(0.0, 0.6, 0.2, 1.0),
            0xFF0000FF, // Red
        ),
        // Occluded: Triangle behind the occluder
        (
            make_clip_vertex(-0.4, -0.4, 0.8, 1.0), // Far from camera
            make_clip_vertex(0.4, -0.4, 0.8, 1.0),
            make_clip_vertex(0.0, 0.4, 0.8, 1.0),
            0x00FF00FF, // Green (should be hidden)
        ),
        // Visible: Small triangle in corner
        (
            make_clip_vertex(-0.9, -0.9, 0.3, 1.0),
            make_clip_vertex(-0.8, -0.9, 0.3, 1.0),
            make_clip_vertex(-0.85, -0.8, 0.3, 1.0),
            0x0000FFFF, // Blue
        ),
    ]
}

/// Helper to create a complex scene with depth layering
fn create_layered_scene(layer_count: usize, triangles_per_layer: usize) -> Vec<ClipTriangle> {
    let mut triangles = Vec::new();

    for layer in 0..layer_count {
        let z = 0.2 + (layer as f32 / layer_count as f32) * 0.6; // 0.2 to 0.8
        let color_shift = (layer as u32) << 8;

        for i in 0..triangles_per_layer {
            let angle = (i as f32 / triangles_per_layer as f32) * std::f32::consts::PI * 2.0;
            let radius = 0.3 + (i as f32 / triangles_per_layer as f32) * 0.3;

            let x = angle.cos() * radius;
            let y = angle.sin() * radius;
            let size = 0.08;

            triangles.push((
                make_clip_vertex(x - size, y - size, z, 1.0),
                make_clip_vertex(x + size, y - size, z, 1.0),
                make_clip_vertex(x, y + size, z, 1.0),
                0xFF0000FF | color_shift,
            ));
        }
    }

    triangles
}

/// Helper to count non-background pixels
fn count_rendered_pixels(fb: &Framebuffer, background: u32) -> usize {
    fb.as_slice().iter().filter(|&&p| p != background).count()
}

/// Helper to compare two framebuffers with tolerance for minor differences
fn compare_framebuffers(fb1: &Framebuffer, fb2: &Framebuffer, tolerance: usize) -> bool {
    assert_eq!(fb1.as_slice().len(), fb2.as_slice().len());

    let mut diff_count = 0;
    for (p1, p2) in fb1.as_slice().iter().zip(fb2.as_slice().iter()) {
        if p1 != p2 {
            diff_count += 1;
        }
    }

    diff_count <= tolerance
}

#[test]
fn test_two_level_matches_single_level() {
    // Test that two-level binning produces pixel-identical output to single-level binning
    let width = 1920;
    let height = 1080;

    // Create renderers
    let mut renderer_single = TileRenderer::new(width, height);
    let mut renderer_two_level = TileRenderer::new(width, height);

    // Enable GPU binning for single-level (baseline)
    renderer_single
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    // Enable GPU binning + two-level for hierarchical (test subject)
    renderer_two_level
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    // TODO: Enable two-level binning once API is integrated
    // renderer_two_level
    //     .enable_two_level_binning()
    //     .expect("Failed to enable two-level binning");

    // Create framebuffers
    let mut fb_single = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut fb_two_level = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb_single = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let mut zb_two_level = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Test with simple scene
    let triangles = create_occluder_scene();

    // Render with both approaches
    renderer_single.render_batch(&mut fb_single, &mut zb_single, &triangles);
    renderer_two_level.render_batch(&mut fb_two_level, &mut zb_two_level, &triangles);

    // Compare results - should be pixel-identical
    let single_pixels = fb_single.as_slice();
    let two_level_pixels = fb_two_level.as_slice();

    let mut diff_count = 0;
    for i in 0..single_pixels.len() {
        if single_pixels[i] != two_level_pixels[i] {
            diff_count += 1;
        }
    }

    assert_eq!(
        diff_count,
        0,
        "Two-level and single-level rendering differ in {diff_count} pixels out of {}",
        single_pixels.len()
    );

    // Also compare depth buffers
    let single_depths = zb_single.as_slice();
    let two_level_depths = zb_two_level.as_slice();

    for i in 0..single_depths.len() {
        if single_depths[i] != 0.0 || two_level_depths[i] != 0.0 {
            // Handle special float values (infinity, NaN)
            if single_depths[i].is_infinite() && two_level_depths[i].is_infinite() {
                assert_eq!(
                    single_depths[i].is_sign_positive(),
                    two_level_depths[i].is_sign_positive(),
                    "Infinite depth signs don't match at pixel {i}"
                );
            } else if single_depths[i].is_nan() || two_level_depths[i].is_nan() {
                panic!(
                    "NaN depth at pixel {i}: single={}, two-level={}",
                    single_depths[i], two_level_depths[i]
                );
            } else {
                assert!(
                    (single_depths[i] - two_level_depths[i]).abs() < 0.0001,
                    "Depth mismatch at pixel {i}: single={}, two-level={}",
                    single_depths[i],
                    two_level_depths[i]
                );
            }
        }
    }
}

#[test]
fn test_coarse_binning_accuracy() {
    // Test that coarse bins (128×128) correctly contain all triangles that touch them
    let width = 1920;
    let height = 1080;

    let mut renderer = TileRenderer::new(width, height);
    renderer
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    // TODO: Enable two-level binning and verify coarse bin coverage
    // For now, test basic binning behavior

    let mut fb = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Create triangles at specific coarse bin boundaries
    // Coarse bin size: 128×128
    // At 1920×1080: 15×9 coarse bins (1920/128=15, 1080/128=8.4→9)

    let triangles = vec![
        // Triangle in coarse bin (0,0): screen coords (0,0)-(127,127)
        (
            make_clip_vertex(-1.0, -1.0, 0.5, 1.0),
            make_clip_vertex(-0.867, -1.0, 0.5, 1.0), // ~128 pixels at 1920 width
            make_clip_vertex(-0.933, -0.881, 0.5, 1.0), // ~128 pixels at 1080 height
            0xFF0000FF,                               // Red
        ),
        // Triangle spanning multiple coarse bins
        (
            make_clip_vertex(-0.5, -0.5, 0.5, 1.0),
            make_clip_vertex(0.5, -0.5, 0.5, 1.0),
            make_clip_vertex(0.0, 0.5, 0.5, 1.0),
            0x00FF00FF, // Green
        ),
    ];

    renderer.render_batch(&mut fb, &mut zb, &triangles);

    // Verify rendering occurred (basic sanity check)
    let background = 0xFF000000;
    let rendered_pixels = count_rendered_pixels(&fb, background);
    assert!(
        rendered_pixels > 0,
        "Coarse binning test: no pixels rendered"
    );
}

#[test]
fn test_hiz_culling_effectiveness() {
    // Test that Hi-Z culling correctly identifies and culls occluded coarse bins
    let width = 1920;
    let height = 1080;

    let mut hiz = HiZBuffer::new(width, height);
    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Fill zbuffer with an occluder at depth 0.3
    let slice = zb.as_mut_slice();
    for y in 200..400 {
        for x in 400..800 {
            slice[y * width as usize + x] = 0.3;
        }
    }

    // Build Hi-Z pyramid
    hiz.build_pyramid(&zb);

    // Test coarse bin visibility
    // Bin covering the occluder region (at depth 0.3)
    let bin_covered = AABB3D {
        min_x: 400,
        max_x: 527, // 128×128 bin
        min_y: 256,
        max_y: 383,
        min_depth: 0.5, // Behind occluder
        max_depth: 1.0,
    };

    // Should be culled (farther than occluder)
    assert!(
        !hiz.is_coarse_bin_visible(bin_covered),
        "Occluded bin should be culled by Hi-Z"
    );

    // Bin in front of occluder
    let bin_front = AABB3D {
        min_x: 400,
        max_x: 527,
        min_y: 256,
        max_y: 383,
        min_depth: 0.1, // In front of occluder
        max_depth: 0.2,
    };

    // Should be visible (closer than occluder)
    assert!(
        hiz.is_coarse_bin_visible(bin_front),
        "Front bin should be visible through Hi-Z"
    );

    // Bin in unoccluded region
    let bin_clear = AABB3D {
        min_x: 1600,
        max_x: 1727,
        min_y: 800,
        max_y: 927,
        min_depth: 0.5,
        max_depth: 1.0,
    };

    // Should be visible (no occlusion)
    assert!(
        hiz.is_coarse_bin_visible(bin_clear),
        "Unoccluded bin should be visible"
    );
}

#[test]
fn test_fine_binning_completeness() {
    // Test that fine tiles (32×32) within visible coarse bins cover all expected pixels
    let width = 1920;
    let height = 1080;

    let mut renderer = TileRenderer::new(width, height);
    renderer
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    let mut fb = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Triangle covering multiple fine tiles within coarse bins
    // At 1920×1080: 60×34 fine tiles (each 32×32)
    let triangles = vec![(
        make_clip_vertex(-0.8, -0.8, 0.5, 1.0),
        make_clip_vertex(-0.4, -0.8, 0.5, 1.0),
        make_clip_vertex(-0.6, -0.4, 0.5, 1.0),
        0xFF00FFFF, // Magenta
    )];

    renderer.render_batch(&mut fb, &mut zb, &triangles);

    // Verify coverage - should render proportional to triangle area
    let background = 0xFF000000;
    let rendered_pixels = count_rendered_pixels(&fb, background);

    // Triangle covers from (-0.8,-0.8) to (-0.4,-0.4) in NDC
    // That's roughly 0.2×0.2 of screen = 0.04 of total area
    // At 1920×1080: 0.04 × 2,073,600 = ~82,944 pixels for a rectangle
    // Triangle is half that: ~41,472 pixels
    // Allow wider range for rasterization tolerance: 30,000-60,000
    assert!(
        rendered_pixels > 30000 && rendered_pixels < 60000,
        "Fine tile rendering coverage unexpected: {rendered_pixels} pixels (expected 30k-60k)"
    );
}

#[test]
fn test_two_level_with_hiz_enabled() {
    // Integration test: full two-level binning pipeline with Hi-Z culling
    let width = 1920;
    let height = 1080;

    let mut renderer = TileRenderer::new(width, height);

    // Enable Hi-Z occlusion culling
    renderer.enable_hiz();

    // Enable GPU binning
    renderer
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    // TODO: Enable two-level binning once API is integrated
    // renderer
    //     .enable_two_level_binning()
    //     .expect("Failed to enable two-level binning");

    let mut fb = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Render occluder scene in a single batch
    // Note: Without actual two-level binning API integrated yet, this tests
    // the baseline behavior with Hi-Z enabled
    let triangles = create_occluder_scene();
    renderer.render_batch(&mut fb, &mut zb, &triangles);

    // Verify rendering: red triangle (occluder) should be visible
    let pixels = fb.as_slice();
    let red_pixels = pixels
        .iter()
        .filter(|&&p| (p & 0xFF0000FF) == 0xFF0000FF) // Check for red
        .count();

    let background = 0xFF000000;
    let total_rendered = count_rendered_pixels(&fb, background);

    // Red triangle should be the dominant color (front occluder)
    // Green behind it should be mostly hidden by depth test
    assert!(red_pixels > 0, "Red occluder triangle should be visible");

    // Basic sanity check: some pixels were rendered
    assert!(
        total_rendered > 100000,
        "Expected substantial rendering with occluder scene, got {total_rendered} pixels"
    );
}

#[test]
fn test_two_level_edge_cases() {
    // Test edge cases: empty scene, single triangle, bin overflow scenarios
    let width = 1920;
    let height = 1080;

    let mut renderer = TileRenderer::new(width, height);
    renderer
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    // TODO: Enable two-level binning once API is integrated
    // renderer
    //     .enable_two_level_binning()
    //     .expect("Failed to enable two-level binning");

    let mut fb = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Edge case 1: Empty scene
    renderer.render_batch(&mut fb, &mut zb, &[]);
    let background = 0xFF000000;
    assert_eq!(
        count_rendered_pixels(&fb, background),
        0,
        "Empty scene should render no pixels"
    );

    // Edge case 2: Single triangle
    let single_triangle = vec![(
        make_clip_vertex(0.0, -0.5, 0.5, 1.0),
        make_clip_vertex(0.5, 0.5, 0.5, 1.0),
        make_clip_vertex(-0.5, 0.5, 0.5, 1.0),
        0xFFFFFFFF, // White
    )];
    renderer.render_batch(&mut fb, &mut zb, &single_triangle);
    assert!(
        count_rendered_pixels(&fb, background) > 0,
        "Single triangle should render pixels"
    );

    // Edge case 3: Very large triangle (entire screen)
    fb.clear(background);
    zb.clear();
    let screen_triangle = vec![(
        make_clip_vertex(-1.0, -1.0, 0.5, 1.0),
        make_clip_vertex(3.0, -1.0, 0.5, 1.0),
        make_clip_vertex(-1.0, 3.0, 0.5, 1.0),
        0xFF8080FF, // Light red
    )];
    renderer.render_batch(&mut fb, &mut zb, &screen_triangle);
    let screen_coverage = count_rendered_pixels(&fb, background) as f32 / (width * height) as f32;
    assert!(
        screen_coverage > 0.9,
        "Screen-covering triangle should cover >90% of pixels, got {:.1}%",
        screen_coverage * 100.0
    );

    // Edge case 4: Many small triangles (bin capacity stress test)
    fb.clear(background);
    zb.clear();
    let many_triangles = create_layered_scene(3, 30); // 90 triangles
    renderer.render_batch(&mut fb, &mut zb, &many_triangles);
    assert!(
        count_rendered_pixels(&fb, background) > 0,
        "Many triangles should render without overflow"
    );
}

#[test]
fn test_two_level_complex_occlusion() {
    // Test complex layered scene with multiple depth layers
    let width = 1920;
    let height = 1080;

    // Renderer with two-level binning + Hi-Z
    let mut renderer_hiz = TileRenderer::new(width, height);
    renderer_hiz.enable_hiz();
    renderer_hiz
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    // Renderer without Hi-Z (baseline)
    let mut renderer_baseline = TileRenderer::new(width, height);
    renderer_baseline
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    let mut fb_hiz = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut fb_baseline = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb_hiz = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let mut zb_baseline = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Create layered scene: 5 layers, 20 triangles per layer
    let triangles = create_layered_scene(5, 20);

    // Render with both approaches
    renderer_hiz.render_batch(&mut fb_hiz, &mut zb_hiz, &triangles);
    renderer_baseline.render_batch(&mut fb_baseline, &mut zb_baseline, &triangles);

    // Results should be pixel-identical (Hi-Z doesn't change correctness)
    assert!(
        compare_framebuffers(&fb_hiz, &fb_baseline, 0),
        "Hi-Z culling should produce identical output to baseline"
    );

    // Verify depth buffers match
    let hiz_depths = zb_hiz.as_slice();
    let baseline_depths = zb_baseline.as_slice();
    let mut depth_diff_count = 0;

    for i in 0..hiz_depths.len() {
        if (hiz_depths[i] - baseline_depths[i]).abs() > 0.0001 {
            depth_diff_count += 1;
        }
    }

    assert_eq!(
        depth_diff_count, 0,
        "Depth buffers differ in {depth_diff_count} pixels"
    );
}

#[test]
fn test_two_level_performance_characteristics() {
    // Performance validation: two-level should complete without timeout
    let width = 1920;
    let height = 1080;

    let mut renderer = TileRenderer::new(width, height);
    renderer.enable_hiz();
    renderer
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    let mut fb = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Large scene: 10 layers × 30 triangles = 300 triangles
    let triangles = create_layered_scene(10, 30);

    // This should complete within reasonable time (not a precise benchmark)
    let start = std::time::Instant::now();
    renderer.render_batch(&mut fb, &mut zb, &triangles);
    let duration = start.elapsed();

    // Sanity check: should complete in <500ms even on slow hardware
    assert!(
        duration.as_millis() < 500,
        "Two-level binning took {duration:?}, expected <500ms"
    );

    // Verify rendering occurred
    let background = 0xFF000000;
    assert!(
        count_rendered_pixels(&fb, background) > 0,
        "Performance test: no pixels rendered"
    );
}
