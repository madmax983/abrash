//! Correctness tests for GPU compute binning
//!
//! Verifies that GPU binning produces identical results to CPU binning.

#![cfg(all(feature = "backend-win32", feature = "gpu-binning"))]

use abrash::{
    framebuffer::Framebuffer,
    math::Vec3,
    rasterizer::tile::{ClipTriangle, TileRenderer},
    zbuffer::ZBuffer,
};

/// Helper to create a clip-space vertex
fn make_clip_vertex(x: f32, y: f32, z: f32, w: f32) -> (Vec3, f32) {
    (Vec3::new(x, y, z), w)
}

/// Helper to create a simple test scene
fn create_test_triangles() -> Vec<ClipTriangle> {
    vec![
        // Triangle 1: Large triangle covering multiple tiles
        (
            make_clip_vertex(-0.5, -0.5, 0.5, 1.0),
            make_clip_vertex(0.5, -0.5, 0.5, 1.0),
            make_clip_vertex(0.0, 0.5, 0.5, 1.0),
            0xFF0000FF, // Red
        ),
        // Triangle 2: Small triangle in corner
        (
            make_clip_vertex(-0.9, -0.9, 0.3, 1.0),
            make_clip_vertex(-0.8, -0.9, 0.3, 1.0),
            make_clip_vertex(-0.85, -0.8, 0.3, 1.0),
            0x00FF00FF, // Green
        ),
        // Triangle 3: Overlapping triangle
        (
            make_clip_vertex(-0.3, 0.0, 0.7, 1.0),
            make_clip_vertex(0.3, 0.0, 0.7, 1.0),
            make_clip_vertex(0.0, 0.6, 0.7, 1.0),
            0x0000FFFF, // Blue
        ),
    ]
}

/// Helper to create a complex scene with many triangles
fn create_complex_scene(count: usize) -> Vec<ClipTriangle> {
    let mut triangles = Vec::new();

    for i in 0..count {
        let angle = (i as f32 / count as f32) * std::f32::consts::PI * 2.0;
        let radius = 0.3 + (i as f32 / count as f32) * 0.4;

        let x = angle.cos() * radius;
        let y = angle.sin() * radius;
        let z = 0.5 + (i as f32 / count as f32) * 0.3;

        let size = 0.1;

        triangles.push((
            make_clip_vertex(x - size, y - size, z, 1.0),
            make_clip_vertex(x + size, y - size, z, 1.0),
            make_clip_vertex(x, y + size, z, 1.0),
            0xFF0000FF | ((i as u32) << 8), // Varying colors
        ));
    }

    triangles
}

#[test]
fn test_gpu_binning_matches_cpu_single_triangle() {
    let width = 800;
    let height = 600;

    // Create renderers
    let mut renderer_cpu = TileRenderer::new(width, height);
    let mut renderer_gpu = TileRenderer::new(width, height);

    // Enable GPU binning
    renderer_gpu
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    // Create framebuffers
    let mut fb_cpu = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut fb_gpu = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb_cpu = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let mut zb_gpu = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Single triangle test
    let triangles = vec![(
        make_clip_vertex(-0.5, -0.5, 0.5, 1.0),
        make_clip_vertex(0.5, -0.5, 0.5, 1.0),
        make_clip_vertex(0.0, 0.5, 0.5, 1.0),
        0xFF0000FF,
    )];

    // Render with both
    renderer_cpu.render_batch(&mut fb_cpu, &mut zb_cpu, &triangles);
    renderer_gpu.render_batch(&mut fb_gpu, &mut zb_gpu, &triangles);

    // Compare results - should be pixel-identical
    let cpu_pixels = fb_cpu.as_slice();
    let gpu_pixels = fb_gpu.as_slice();

    let mut diff_count = 0;
    for i in 0..cpu_pixels.len() {
        if cpu_pixels[i] != gpu_pixels[i] {
            diff_count += 1;
        }
    }

    assert_eq!(
        diff_count,
        0,
        "GPU and CPU rendering differ in {diff_count} pixels out of {}",
        cpu_pixels.len()
    );

    // Also compare depth buffers
    let cpu_depths = zb_cpu.as_slice();
    let gpu_depths = zb_gpu.as_slice();

    for i in 0..cpu_depths.len() {
        if cpu_depths[i] != 0.0 || gpu_depths[i] != 0.0 {
            // Handle special float values (infinity, NaN)
            if cpu_depths[i].is_infinite() && gpu_depths[i].is_infinite() {
                assert_eq!(
                    cpu_depths[i].is_sign_positive(),
                    gpu_depths[i].is_sign_positive(),
                    "Infinite depth signs don't match at pixel {i}"
                );
            } else if cpu_depths[i].is_nan() || gpu_depths[i].is_nan() {
                panic!(
                    "NaN depth at pixel {i}: CPU={}, GPU={}",
                    cpu_depths[i], gpu_depths[i]
                );
            } else {
                assert!(
                    (cpu_depths[i] - gpu_depths[i]).abs() < 0.0001,
                    "Depth mismatch at pixel {i}: CPU={}, GPU={}",
                    cpu_depths[i],
                    gpu_depths[i]
                );
            }
        }
    }
}

#[test]
fn test_gpu_binning_matches_cpu_multiple_triangles() {
    let width = 1024;
    let height = 768;

    let mut renderer_cpu = TileRenderer::new(width, height);
    let mut renderer_gpu = TileRenderer::new(width, height);

    renderer_gpu
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    let mut fb_cpu = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut fb_gpu = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb_cpu = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let mut zb_gpu = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Multiple overlapping triangles
    let triangles = create_test_triangles();

    renderer_cpu.render_batch(&mut fb_cpu, &mut zb_cpu, &triangles);
    renderer_gpu.render_batch(&mut fb_gpu, &mut zb_gpu, &triangles);

    // Compare results
    let cpu_pixels = fb_cpu.as_slice();
    let gpu_pixels = fb_gpu.as_slice();

    let mut diff_count = 0;
    for i in 0..cpu_pixels.len() {
        if cpu_pixels[i] != gpu_pixels[i] {
            diff_count += 1;
        }
    }

    assert_eq!(
        diff_count, 0,
        "GPU and CPU rendering differ in {diff_count} pixels"
    );
}

#[test]
fn test_gpu_binning_matches_cpu_complex_scene() {
    let width = 1920;
    let height = 1080;

    let mut renderer_cpu = TileRenderer::new(width, height);
    let mut renderer_gpu = TileRenderer::new(width, height);

    renderer_gpu
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    let mut fb_cpu = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut fb_gpu = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb_cpu = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let mut zb_gpu = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Complex scene with 50 triangles
    let triangles = create_complex_scene(50);

    renderer_cpu.render_batch(&mut fb_cpu, &mut zb_cpu, &triangles);
    renderer_gpu.render_batch(&mut fb_gpu, &mut zb_gpu, &triangles);

    // Compare results
    let cpu_pixels = fb_cpu.as_slice();
    let gpu_pixels = fb_gpu.as_slice();

    let mut diff_count = 0;
    for i in 0..cpu_pixels.len() {
        if cpu_pixels[i] != gpu_pixels[i] {
            diff_count += 1;
        }
    }

    assert_eq!(
        diff_count, 0,
        "GPU and CPU rendering differ in {diff_count} pixels with 50 triangles"
    );
}

#[test]
fn test_gpu_binning_triangle_heavy_scene() {
    let width = 1920;
    let height = 1080;

    let mut renderer_cpu = TileRenderer::new(width, height);
    let mut renderer_gpu = TileRenderer::new(width, height);

    renderer_gpu
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    let mut fb_cpu = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut fb_gpu = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb_cpu = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let mut zb_gpu = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Heavy scene with 75 triangles (ensure no bin overflow)
    let triangles = create_complex_scene(75);

    renderer_cpu.render_batch(&mut fb_cpu, &mut zb_cpu, &triangles);
    renderer_gpu.render_batch(&mut fb_gpu, &mut zb_gpu, &triangles);

    // Compare results
    let cpu_pixels = fb_cpu.as_slice();
    let gpu_pixels = fb_gpu.as_slice();

    let mut diff_count = 0;
    for i in 0..cpu_pixels.len() {
        if cpu_pixels[i] != gpu_pixels[i] {
            diff_count += 1;
        }
    }

    assert_eq!(
        diff_count, 0,
        "GPU and CPU rendering differ in {diff_count} pixels with 75 triangles"
    );
}

#[test]
fn test_gpu_binning_edge_cases() {
    let width = 640;
    let height = 480;

    let mut renderer_gpu = TileRenderer::new(width, height);
    renderer_gpu
        .enable_gpu_binning()
        .expect("Failed to enable GPU binning");

    let mut fb = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Edge case: Empty scene
    renderer_gpu.render_batch(&mut fb, &mut zb, &[]);

    // Edge case: Single pixel triangle
    let tiny_triangle = vec![(
        make_clip_vertex(0.0, 0.0, 0.5, 1.0),
        make_clip_vertex(0.001, 0.0, 0.5, 1.0),
        make_clip_vertex(0.0, 0.001, 0.5, 1.0),
        0xFFFFFFFF,
    )];
    renderer_gpu.render_batch(&mut fb, &mut zb, &tiny_triangle);

    // Edge case: Very large triangle covering entire screen
    let large_triangle = vec![(
        make_clip_vertex(-1.0, -1.0, 0.5, 1.0),
        make_clip_vertex(1.0, -1.0, 0.5, 1.0),
        make_clip_vertex(0.0, 1.0, 0.5, 1.0),
        0xFF00FFFF,
    )];
    renderer_gpu.render_batch(&mut fb, &mut zb, &large_triangle);

    // If we got here without panicking, edge cases handled correctly
}

#[test]
fn test_gpu_binning_fallback_on_error() {
    let width = 800;
    let height = 600;

    // This test verifies that even if GPU initialization fails,
    // rendering still works via CPU fallback
    let mut renderer = TileRenderer::new(width, height);

    // Try to enable GPU binning (might fail on some systems)
    let _ = renderer.enable_gpu_binning();

    let mut fb = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    let triangles = create_test_triangles();

    // Should work regardless of GPU availability
    renderer.render_batch(&mut fb, &mut zb, &triangles);

    // Verify some pixels were drawn
    let pixels = fb.as_slice();
    let non_black = pixels.iter().filter(|&&p| p != 0xFF000000).count();

    assert!(non_black > 0, "No pixels were rendered");
}
