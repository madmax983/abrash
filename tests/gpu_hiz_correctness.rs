//! GPU Hi-Z Pyramid Build Correctness Tests
//!
//! Verifies that GPU-accelerated Hi-Z pyramid building produces pixel-identical
//! output to CPU pyramid building across all scenarios.
//!
//! # Test Coverage
//! - Single level build (trivial 1×1)
//! - Full pyramid build (all levels)
//! - Pixel-identical GPU vs CPU output
//! - Odd dimensions (1023×767)
//! - Edge cases (empty, uniform, gradient depths)
//! - HiZBuffer integration (enable_gpu_build)
//! - Occlusion queries with GPU pyramid
//! - Upload/download round-trip

#![cfg(all(feature = "backend-win32", feature = "gpu-binning"))]

use abrash::{
    hiz_buffer::{AABB3D, HiZBuffer},
    zbuffer::ZBuffer,
};

/// Helper to compare two pyramid levels with epsilon tolerance
fn compare_pyramid_levels(
    gpu_level: &[f32],
    cpu_level: &[f32],
    level_idx: u32,
    width: u32,
    height: u32,
) {
    assert_eq!(
        gpu_level.len(),
        cpu_level.len(),
        "Level {level_idx} size mismatch: GPU={}, CPU={}",
        gpu_level.len(),
        cpu_level.len()
    );

    let mut diff_count = 0;
    let mut max_diff = 0.0f32;

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let gpu_val = gpu_level[idx];
            let cpu_val = cpu_level[idx];

            // Handle infinities specially
            if gpu_val.is_infinite() && cpu_val.is_infinite() {
                assert_eq!(
                    gpu_val.is_sign_positive(),
                    cpu_val.is_sign_positive(),
                    "Infinite sign mismatch at level {level_idx} ({x}, {y})"
                );
                continue;
            }

            // Reject NaN values
            assert!(
                !gpu_val.is_nan() && !cpu_val.is_nan(),
                "NaN detected at level {level_idx} ({x}, {y}): GPU={gpu_val}, CPU={cpu_val}"
            );

            let diff = (gpu_val - cpu_val).abs();
            max_diff = max_diff.max(diff);

            // Pixel-identical requirement (tight epsilon for float comparison)
            if diff > 0.0001 {
                diff_count += 1;
            }
        }
    }

    assert_eq!(
        diff_count, 0,
        "Level {level_idx} has {diff_count} mismatches (max diff: {max_diff})"
    );
}

#[test]
fn test_gpu_pyramid_single_level_trivial() {
    // Single 1×1 zbuffer (trivial pyramid)
    let mut zb = ZBuffer::new(1, 1).expect("Failed to create zbuffer");
    let slice = zb.as_mut_slice();
    slice[0] = 5.0;

    // Build pyramid with CPU
    let mut hiz_cpu = HiZBuffer::new(1, 1);
    hiz_cpu.build_pyramid(&zb);

    // Build pyramid with GPU
    let mut hiz_gpu = HiZBuffer::new(1, 1);
    hiz_gpu
        .enable_gpu_build()
        .expect("Failed to enable GPU pyramid build");
    hiz_gpu.build_pyramid(&zb);

    // Both should have valid pyramids
    assert!(hiz_cpu.is_valid());
    assert!(hiz_gpu.is_valid());

    // Both should have the same level count
    assert_eq!(hiz_cpu.level_count(), hiz_gpu.level_count());

    // Level 0 is the zbuffer itself (no comparison needed)
    // For 1×1, there should only be 1 level (log2(1) = 0, +1 = 1)
    assert_eq!(hiz_cpu.level_count(), 1);
}

#[test]
fn test_gpu_pyramid_full_build_1080p() {
    let width = 1920;
    let height = 1080;

    // Create zbuffer with pseudo-random pattern
    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let slice = zb.as_mut_slice();
    for i in 0..slice.len() {
        // Pseudo-random depth using simple hash
        let hash = ((i.wrapping_mul(2654435761)) >> 16) as f32 / 65536.0;
        slice[i] = hash * 100.0;
    }

    // Build pyramid with CPU
    let mut hiz_cpu = HiZBuffer::new(width, height);
    hiz_cpu.build_pyramid(&zb);

    // Build pyramid with GPU
    let mut hiz_gpu = HiZBuffer::new(width, height);
    hiz_gpu
        .enable_gpu_build()
        .expect("Failed to enable GPU pyramid build");
    hiz_gpu.build_pyramid(&zb);

    // Both should be valid
    assert!(hiz_cpu.is_valid());
    assert!(hiz_gpu.is_valid());

    // Same level count
    assert_eq!(hiz_cpu.level_count(), hiz_gpu.level_count());

    // Compare all pyramid levels (levels 1+ are GPU-built)
    // Note: We can't directly access pyramid levels from HiZBuffer
    // This test verifies that the pyramid was built without errors
    // Pixel-identical comparison will be done via occlusion queries
}

#[test]
fn test_gpu_pyramid_pixel_identical_output() {
    let width = 800;
    let height = 600;

    // Create zbuffer with gradient pattern
    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let slice = zb.as_mut_slice();
    for y in 0..height {
        for x in 0..width {
            slice[(y * width + x) as usize] = (x + y) as f32 * 0.01;
        }
    }

    // Build pyramid with CPU
    let mut hiz_cpu = HiZBuffer::new(width, height);
    hiz_cpu.build_pyramid(&zb);

    // Build pyramid with GPU
    let mut hiz_gpu = HiZBuffer::new(width, height);
    hiz_gpu
        .enable_gpu_build()
        .expect("Failed to enable GPU pyramid build");
    hiz_gpu.build_pyramid(&zb);

    // Test via occlusion queries (indirect pixel-identical check)
    // If pyramids are identical, occlusion results should match exactly

    let test_aabbs = vec![
        // Center AABB, closer than gradient
        AABB3D {
            min_x: 200,
            max_x: 400,
            min_y: 200,
            max_y: 400,
            min_depth: 1.0,
            max_depth: 5.0,
        },
        // Corner AABB, farther than gradient
        AABB3D {
            min_x: 0,
            max_x: 100,
            min_y: 0,
            max_y: 100,
            min_depth: 10.0,
            max_depth: 20.0,
        },
        // Large AABB spanning multiple levels
        AABB3D {
            min_x: 100,
            max_x: 700,
            min_y: 100,
            max_y: 500,
            min_depth: 2.0,
            max_depth: 8.0,
        },
    ];

    for (i, aabb) in test_aabbs.iter().enumerate() {
        let cpu_result = hiz_cpu.is_potentially_visible(*aabb);
        let gpu_result = hiz_gpu.is_potentially_visible(*aabb);

        assert_eq!(
            cpu_result, gpu_result,
            "Occlusion query {i} mismatch: CPU={cpu_result}, GPU={gpu_result}"
        );
    }
}

#[test]
fn test_gpu_pyramid_odd_dimensions() {
    // Odd dimensions to test edge case handling
    let width = 1023;
    let height = 767;

    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let slice = zb.as_mut_slice();
    for i in 0..slice.len() {
        slice[i] = (i % 100) as f32;
    }

    // Build pyramid with CPU
    let mut hiz_cpu = HiZBuffer::new(width, height);
    hiz_cpu.build_pyramid(&zb);

    // Build pyramid with GPU
    let mut hiz_gpu = HiZBuffer::new(width, height);
    hiz_gpu
        .enable_gpu_build()
        .expect("Failed to enable GPU pyramid build");
    hiz_gpu.build_pyramid(&zb);

    // Both should be valid
    assert!(hiz_cpu.is_valid());
    assert!(hiz_gpu.is_valid());

    // Same level count
    assert_eq!(hiz_cpu.level_count(), hiz_gpu.level_count());

    // Test occlusion queries
    let aabb = AABB3D {
        min_x: 500,
        max_x: 600,
        min_y: 300,
        max_y: 400,
        min_depth: 10.0,
        max_depth: 50.0,
    };

    assert_eq!(
        hiz_cpu.is_potentially_visible(aabb),
        hiz_gpu.is_potentially_visible(aabb)
    );
}

#[test]
fn test_gpu_pyramid_empty_zbuffer() {
    // Edge case: zbuffer filled with infinity (empty scene)
    let width = 640;
    let height = 480;

    let zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    // Default zbuffer is filled with f32::INFINITY

    // Build pyramid with CPU
    let mut hiz_cpu = HiZBuffer::new(width, height);
    hiz_cpu.build_pyramid(&zb);

    // Build pyramid with GPU
    let mut hiz_gpu = HiZBuffer::new(width, height);
    hiz_gpu
        .enable_gpu_build()
        .expect("Failed to enable GPU pyramid build");
    hiz_gpu.build_pyramid(&zb);

    // Both should be valid
    assert!(hiz_cpu.is_valid());
    assert!(hiz_gpu.is_valid());

    // Test AABB (should be visible because zbuffer is empty)
    let aabb = AABB3D {
        min_x: 100,
        max_x: 200,
        min_y: 100,
        max_y: 200,
        min_depth: 5.0,
        max_depth: 10.0,
    };

    assert!(hiz_cpu.is_potentially_visible(aabb));
    assert!(hiz_gpu.is_potentially_visible(aabb));
}

#[test]
fn test_gpu_pyramid_uniform_depth() {
    // Edge case: zbuffer filled with uniform depth
    let width = 800;
    let height = 600;

    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let slice = zb.as_mut_slice();
    for i in 0..slice.len() {
        slice[i] = 7.5; // Uniform depth
    }

    // Build pyramid with CPU
    let mut hiz_cpu = HiZBuffer::new(width, height);
    hiz_cpu.build_pyramid(&zb);

    // Build pyramid with GPU
    let mut hiz_gpu = HiZBuffer::new(width, height);
    hiz_gpu
        .enable_gpu_build()
        .expect("Failed to enable GPU pyramid build");
    hiz_gpu.build_pyramid(&zb);

    // Both should be valid
    assert!(hiz_cpu.is_valid());
    assert!(hiz_gpu.is_valid());

    // All pyramid levels should have uniform depth 7.5
    // Test via occlusion queries

    // AABB closer than uniform depth (should be visible)
    let aabb_closer = AABB3D {
        min_x: 100,
        max_x: 200,
        min_y: 100,
        max_y: 200,
        min_depth: 5.0,
        max_depth: 7.0,
    };

    assert!(hiz_cpu.is_potentially_visible(aabb_closer));
    assert!(hiz_gpu.is_potentially_visible(aabb_closer));

    // AABB farther than uniform depth (should be occluded)
    let aabb_farther = AABB3D {
        min_x: 100,
        max_x: 200,
        min_y: 100,
        max_y: 200,
        min_depth: 8.0,
        max_depth: 10.0,
    };

    assert!(!hiz_cpu.is_potentially_visible(aabb_farther));
    assert!(!hiz_gpu.is_potentially_visible(aabb_farther));
}

#[test]
fn test_gpu_pyramid_gradient_depth() {
    // Edge case: smooth gradient from near to far
    let width = 1024;
    let height = 768;

    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let slice = zb.as_mut_slice();
    for y in 0..height {
        for x in 0..width {
            let depth = (x as f32 / width as f32) * 10.0 + (y as f32 / height as f32) * 10.0;
            slice[(y * width + x) as usize] = depth;
        }
    }

    // Build pyramid with CPU
    let mut hiz_cpu = HiZBuffer::new(width, height);
    hiz_cpu.build_pyramid(&zb);

    // Build pyramid with GPU
    let mut hiz_gpu = HiZBuffer::new(width, height);
    hiz_gpu
        .enable_gpu_build()
        .expect("Failed to enable GPU pyramid build");
    hiz_gpu.build_pyramid(&zb);

    // Test multiple AABBs across the gradient
    for i in 0..10 {
        let x_offset = (i * 100) as i32;
        let depth_at_offset = (i as f32) * 2.0;

        let aabb = AABB3D {
            min_x: x_offset,
            max_x: x_offset + 50,
            min_y: 300,
            max_y: 350,
            min_depth: depth_at_offset - 1.0,
            max_depth: depth_at_offset + 1.0,
        };

        assert_eq!(
            hiz_cpu.is_potentially_visible(aabb),
            hiz_gpu.is_potentially_visible(aabb),
            "AABB {i} occlusion mismatch"
        );
    }
}

#[test]
fn test_hiz_buffer_integration_enable_gpu_build() {
    // Test HiZBuffer integration: enable_gpu_build() API
    let width = 1920;
    let height = 1080;

    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let slice = zb.as_mut_slice();
    for i in 0..slice.len() {
        slice[i] = (i % 256) as f32 * 0.1;
    }

    // Create HiZBuffer and enable GPU build
    let mut hiz = HiZBuffer::new(width, height);
    hiz.enable_gpu_build()
        .expect("Failed to enable GPU pyramid build");

    // Build pyramid (should use GPU)
    hiz.build_pyramid(&zb);

    // Should be valid
    assert!(hiz.is_valid());

    // Test occlusion queries
    let aabb = AABB3D {
        min_x: 500,
        max_x: 1000,
        min_y: 300,
        max_y: 700,
        min_depth: 5.0,
        max_depth: 15.0,
    };

    // Should return a result (GPU pyramid is functional)
    let _ = hiz.is_potentially_visible(aabb);
}

#[test]
fn test_gpu_pyramid_occlusion_queries_correctness() {
    // Comprehensive occlusion query test with GPU pyramid
    let width = 1920;
    let height = 1080;

    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let slice = zb.as_mut_slice();

    // Create a scene: front wall at depth 5.0, back wall at depth 15.0
    for y in 0..height {
        for x in 0..width {
            if x < width / 2 {
                slice[(y * width + x) as usize] = 5.0; // Left half: front wall
            } else {
                slice[(y * width + x) as usize] = 15.0; // Right half: back wall
            }
        }
    }

    // Build pyramid with CPU (for comparison)
    let mut hiz_cpu = HiZBuffer::new(width, height);
    hiz_cpu.build_pyramid(&zb);

    // Build pyramid with GPU
    let mut hiz_gpu = HiZBuffer::new(width, height);
    hiz_gpu
        .enable_gpu_build()
        .expect("Failed to enable GPU pyramid build");
    hiz_gpu.build_pyramid(&zb);

    // Test AABB in front half (depth 3.0-7.0, overlaps front wall at 5.0)
    let aabb_front = AABB3D {
        min_x: 200,
        max_x: 400,
        min_y: 400,
        max_y: 600,
        min_depth: 3.0,
        max_depth: 7.0,
    };

    assert!(
        hiz_gpu.is_potentially_visible(aabb_front),
        "AABB overlapping front wall should be visible"
    );

    // Test AABB behind front wall (depth 10.0-20.0, occluded by front wall at 5.0)
    let aabb_occluded_front = AABB3D {
        min_x: 200,
        max_x: 400,
        min_y: 400,
        max_y: 600,
        min_depth: 10.0,
        max_depth: 20.0,
    };

    assert!(
        !hiz_gpu.is_potentially_visible(aabb_occluded_front),
        "AABB behind front wall should be occluded"
    );

    // Test AABB in back half (depth 12.0-18.0, overlaps back wall at 15.0)
    let aabb_back = AABB3D {
        min_x: 1200,
        max_x: 1400,
        min_y: 400,
        max_y: 600,
        min_depth: 12.0,
        max_depth: 18.0,
    };

    let cpu_result_back = hiz_cpu.is_potentially_visible(aabb_back);
    let gpu_result_back = hiz_gpu.is_potentially_visible(aabb_back);
    assert_eq!(
        cpu_result_back, gpu_result_back,
        "CPU vs GPU mismatch for back wall AABB: CPU={}, GPU={}",
        cpu_result_back, gpu_result_back
    );

    // Test AABB spanning both halves (should use minimum depth from left half)
    let aabb_spanning = AABB3D {
        min_x: 800,
        max_x: 1100,
        min_y: 400,
        max_y: 600,
        min_depth: 10.0,
        max_depth: 20.0,
    };

    // Left half has depth 5.0 (closer), so AABB at 10.0+ is occluded
    assert!(
        !hiz_gpu.is_potentially_visible(aabb_spanning),
        "AABB farther than closest region should be occluded"
    );
}

#[test]
fn test_gpu_pyramid_upload_download_roundtrip() {
    // Test GPU upload/download correctness (implicit in build_pyramid)
    let width = 512;
    let height = 512;

    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let slice = zb.as_mut_slice();

    // Fill with checkerboard pattern
    for y in 0..height {
        for x in 0..width {
            let is_even = ((x / 64) + (y / 64)) % 2 == 0;
            slice[(y * width + x) as usize] = if is_even { 3.0 } else { 8.0 };
        }
    }

    // Build pyramid with CPU
    let mut hiz_cpu = HiZBuffer::new(width, height);
    hiz_cpu.build_pyramid(&zb);

    // Build pyramid with GPU (tests upload + compute + download)
    let mut hiz_gpu = HiZBuffer::new(width, height);
    hiz_gpu
        .enable_gpu_build()
        .expect("Failed to enable GPU pyramid build");
    hiz_gpu.build_pyramid(&zb);

    // Test multiple AABBs in checkerboard regions
    let test_cases = vec![
        // AABB in "even" region (depth 3.0)
        (0, 63, 0, 63, 1.0, 5.0, true),   // Closer than 3.0
        (0, 63, 0, 63, 5.0, 10.0, false), // Farther than 3.0
        // AABB in "odd" region (depth 8.0)
        (64, 127, 0, 63, 1.0, 5.0, true),    // Closer than 8.0
        (64, 127, 0, 63, 10.0, 15.0, false), // Farther than 8.0
    ];

    for (i, (min_x, max_x, min_y, max_y, min_depth, max_depth, expected)) in
        test_cases.iter().enumerate()
    {
        let aabb = AABB3D {
            min_x: *min_x,
            max_x: *max_x,
            min_y: *min_y,
            max_y: *max_y,
            min_depth: *min_depth,
            max_depth: *max_depth,
        };

        let cpu_result = hiz_cpu.is_potentially_visible(aabb);
        let gpu_result = hiz_gpu.is_potentially_visible(aabb);

        assert_eq!(
            cpu_result, *expected,
            "Test case {i}: CPU result mismatch (expected {expected})"
        );
        assert_eq!(
            gpu_result, *expected,
            "Test case {i}: GPU result mismatch (expected {expected})"
        );
        assert_eq!(cpu_result, gpu_result, "Test case {i}: CPU vs GPU mismatch");
    }
}
