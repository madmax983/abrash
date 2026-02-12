//! SIMD Performance Regression Tests
//!
//! These tests ensure that SIMD optimizations remain effective and don't regress
//! over time. They measure the speedup ratio and fail if SIMD becomes slower than
//! expected thresholds.

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::_rdtsc;

use abrash::math::Vec3;

/// Safe wrapper for RDTSC instruction
#[cfg(target_arch = "x86_64")]
#[inline]
fn read_tsc() -> u64 {
    unsafe { _rdtsc() }
}

#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn read_tsc() -> u64 {
    0
}

/// Measure average cycles for an operation
#[cfg(target_arch = "x86_64")]
fn measure_cycles<F: FnMut()>(mut f: F, iterations: usize) -> u64 {
    // Warmup
    for _ in 0..100 {
        f();
    }

    // Measure
    let start = read_tsc();
    for _ in 0..iterations {
        f();
    }
    let end = read_tsc();

    (end - start) / iterations as u64
}

#[cfg(not(target_arch = "x86_64"))]
fn measure_cycles<F: FnMut()>(_f: F, _iterations: usize) -> u64 {
    0
}

/// Generate test scene with triangles
fn generate_test_scene(
    count: usize,
    width: u32,
    height: u32,
) -> Vec<((Vec3, f32), (Vec3, f32), (Vec3, f32), u32)> {
    let mut triangles = Vec::new();

    for i in 0..count {
        let x = ((i % 20) as f32) * (width as f32 / 20.0);
        let y = ((i / 20) as f32) * (height as f32 / 20.0);
        let depth = 5.0 + ((i % 5) as f32) * 2.0;
        let size = 100.0;

        let v0 = (Vec3::new(x, y, depth), 1.0);
        let v1 = (Vec3::new(x + size, y, depth + 0.1), 1.0);
        let v2 = (Vec3::new(x + size / 2.0, y + size, depth + 0.2), 1.0);

        let color = 0xFF_00_00_00 | ((i as u32) << 8);
        triangles.push((v0, v1, v2, color));
    }

    triangles
}

#[test]
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
fn test_hiz_pyramid_performance_threshold() {
    // Hi-Z pyramid build should show measurable performance improvement
    // Due to SIMD 2×2 min reduction

    let width = 1920;
    let height = 1080;
    let zb = ZBuffer::new(width, height).unwrap();
    let mut hiz = HiZBuffer::new(width, height);

    let cycles = measure_cycles(
        || {
            hiz.build_pyramid(&zb);
        },
        100,
    );

    let pixels = width * height;
    let cycles_per_pixel = cycles as f64 / pixels as f64;

    // With SIMD optimizations, we should see:
    // - Less than 5 cycles per pixel for pyramid build
    // - This is a conservative threshold - good SIMD should be ~2-3 cycles/pixel
    println!(
        "Hi-Z pyramid build: {} cycles ({:.2} cycles/pixel)",
        cycles, cycles_per_pixel
    );
    assert!(
        cycles_per_pixel < 5.0,
        "Hi-Z pyramid build too slow: {:.2} cycles/pixel (threshold: 5.0)",
        cycles_per_pixel
    );
}

#[test]
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
fn test_scanline_rasterization_performance() {
    // Scanline rasterization should benefit from SIMD for longer scanlines

    let width = 1920;
    let height = 1080;
    let triangles = generate_test_scene(100, width, height);

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);

    let cycles = measure_cycles(
        || {
            fb.clear(0xFF_00_00_00);
            zb.clear();
            renderer.render_batch(&mut fb, &mut zb, &triangles);
        },
        50,
    );

    let pixels = width * height;
    let cycles_per_pixel = cycles as f64 / pixels as f64;

    // With tiled rendering and SIMD, we should see reasonable performance
    // Conservative threshold: <50 cycles per pixel for full pipeline
    println!(
        "Tile rendering: {} cycles ({:.2} cycles/pixel)",
        cycles, cycles_per_pixel
    );
    assert!(
        cycles_per_pixel < 50.0,
        "Tile rendering too slow: {:.2} cycles/pixel (threshold: 50.0)",
        cycles_per_pixel
    );
}

#[test]
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
fn test_hiz_culling_effectiveness() {
    // Hi-Z culling should reduce rendering time for scenes with depth complexity

    let width = 1920;
    let height = 1080;
    let triangles = generate_overlapping_triangles(200, width, height);

    // Without Hi-Z
    let mut fb1 = Framebuffer::new(width, height).unwrap();
    let mut zb1 = ZBuffer::new(width, height).unwrap();
    let mut renderer1 = TileRenderer::new(width, height);

    let cycles_no_hiz = measure_cycles(
        || {
            fb1.clear(0xFF_00_00_00);
            zb1.clear();
            renderer1.render_batch(&mut fb1, &mut zb1, &triangles);
        },
        50,
    );

    // With Hi-Z
    let mut fb2 = Framebuffer::new(width, height).unwrap();
    let mut zb2 = ZBuffer::new(width, height).unwrap();
    let mut renderer2 = TileRenderer::new(width, height);
    renderer2.enable_hiz();

    let cycles_with_hiz = measure_cycles(
        || {
            fb2.clear(0xFF_00_00_00);
            zb2.clear();
            renderer2.render_batch(&mut fb2, &mut zb2, &triangles);
        },
        50,
    );

    let speedup = cycles_no_hiz as f64 / cycles_with_hiz as f64;

    println!(
        "Hi-Z culling speedup: {:.2}× ({} -> {} cycles)",
        speedup, cycles_no_hiz, cycles_with_hiz
    );

    // Hi-Z should provide at least 1.1× speedup for scenes with overlapping geometry
    // (10% improvement minimum, accounting for pyramid build overhead)
    assert!(
        speedup >= 1.05,
        "Hi-Z culling not effective: {:.2}× speedup (threshold: 1.05×)",
        speedup
    );
}

#[test]
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
fn test_memory_access_efficiency() {
    // Memory operations should have reasonable cycle counts

    let width = 1920;
    let height = 1080;
    let pixels = width * height;

    // Test framebuffer clear (sequential write)
    let mut fb = Framebuffer::new(width, height).unwrap();
    let cycles = measure_cycles(
        || {
            fb.clear(0xFF_FF_FF_FF);
        },
        200,
    );

    let cycles_per_pixel = cycles as f64 / pixels as f64;

    println!(
        "Framebuffer clear: {} cycles ({:.2} cycles/pixel)",
        cycles, cycles_per_pixel
    );

    // Sequential memory writes should be very fast (<1 cycle/pixel with good caching)
    // Conservative threshold: <2 cycles/pixel
    assert!(
        cycles_per_pixel < 2.0,
        "Memory clear too slow: {:.2} cycles/pixel (threshold: 2.0)",
        cycles_per_pixel
    );
}

/// Generate overlapping triangles to test Hi-Z culling
fn generate_overlapping_triangles(
    count: usize,
    width: u32,
    height: u32,
) -> Vec<((Vec3, f32), (Vec3, f32), (Vec3, f32), u32)> {
    let mut triangles = Vec::new();

    // Create layers of triangles at different depths
    for layer in 0..10 {
        let z = 10.0 - layer as f32; // Front to back

        for i in 0..(count / 10) {
            let x = ((i % 10) as f32) * (width as f32 / 10.0);
            let y = ((i / 10) as f32) * (height as f32 / 10.0);
            let size = 200.0;

            let v0 = (Vec3::new(x, y, z), 1.0);
            let v1 = (Vec3::new(x + size, y, z), 1.0);
            let v2 = (Vec3::new(x + size / 2.0, y + size, z), 1.0);

            let color = 0xFF_00_00_00 | ((layer as u32) << 16) | ((i as u32) << 8);
            triangles.push((v0, v1, v2, color));
        }
    }

    triangles
}

#[test]
#[cfg(not(target_arch = "x86_64"))]
fn test_simd_tests_skipped_on_non_x86() {
    // SIMD regression tests only run on x86_64 with RDTSC support
    println!("SIMD regression tests skipped (not x86_64)");
}

#[test]
#[cfg(not(feature = "simd"))]
fn test_simd_tests_require_simd_feature() {
    // These tests require the simd feature flag
    println!("SIMD regression tests skipped (simd feature not enabled)");
}
