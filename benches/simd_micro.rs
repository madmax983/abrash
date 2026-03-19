//! Micro-benchmarks with CPU cycle-level profiling for SIMD operations
//!
//! This benchmark suite measures individual SIMD operations at the cycle level
//! to identify bottlenecks and validate optimization effectiveness.

use abrash::{
    framebuffer::Framebuffer,
    hiz_buffer::HiZBuffer,
    math::Vec3,
    rasterizer::{ClipTriangle, TileRenderer},
    zbuffer::ZBuffer,
};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::_rdtsc;

/// Safe wrapper for RDTSC instruction
#[cfg(target_arch = "x86_64")]
#[inline]
fn read_tsc() -> u64 {
    unsafe { _rdtsc() }
}

#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn read_tsc() -> u64 {
    0 // Fallback for non-x86_64 architectures
}

/// Measure cycles for a single operation
#[cfg(target_arch = "x86_64")]
fn measure_cycles<F: FnMut()>(mut f: F) -> u64 {
    // Warmup
    for _ in 0..100 {
        f();
    }

    // Measure multiple iterations to reduce noise
    let iterations = 1000;
    let start = read_tsc();
    for _ in 0..iterations {
        f();
        black_box(());
    }
    let end = read_tsc();

    (end - start) / iterations
}

#[cfg(not(target_arch = "x86_64"))]
fn measure_cycles<F: FnMut()>(_f: F) -> u64 {
    0
}

/// Benchmark Hi-Z 2×2 reduction with cycle counting
fn bench_hiz_reduction_cycles(c: &mut Criterion) {
    let mut group = c.benchmark_group("hiz_reduction_cycles");

    for resolution in &[(1920, 1080), (3840, 2160)] {
        let (width, height) = *resolution;
        let zb = ZBuffer::new(width, height).unwrap();
        let mut hiz = HiZBuffer::new(width, height);

        group.bench_with_input(
            BenchmarkId::new("pyramid_build", format!("{width}x{height}")),
            resolution,
            |b, _| {
                b.iter(|| {
                    hiz.build_pyramid(black_box(&zb));
                });
            },
        );

        // Report cycles per pixel
        #[cfg(target_arch = "x86_64")]
        {
            let mut hiz_test = HiZBuffer::new(width, height);
            let cycles = measure_cycles(|| {
                hiz_test.build_pyramid(&zb);
            });
            let pixels = width * height;
            let cycles_per_pixel = cycles as f64 / f64::from(pixels);
            eprintln!(
                "Hi-Z {width}×{height}: {cycles} cycles/build ({cycles_per_pixel:.2} cycles/pixel)"
            );
        }
    }

    group.finish();
}

/// Benchmark scanline rasterization with different lengths
fn bench_scanline_rasterization_cycles(c: &mut Criterion) {
    let mut group = c.benchmark_group("scanline_rasterization_cycles");

    // Test different scanline lengths to measure SIMD effectiveness
    for scanline_len in &[4, 8, 16, 32, 64, 128] {
        let triangles = generate_horizontal_triangles(*scanline_len, 10);

        group.bench_with_input(
            BenchmarkId::new("scanline", format!("{scanline_len}_pixels")),
            scanline_len,
            |b, _| {
                let mut fb = Framebuffer::new(1920, 1080).unwrap();
                let mut zb = ZBuffer::new(1920, 1080).unwrap();
                let mut renderer = TileRenderer::new(1920, 1080);

                b.iter(|| {
                    fb.clear(black_box(0xFF_00_00_00));
                    zb.clear();
                    renderer.render_batch(
                        black_box(&mut fb),
                        black_box(&mut zb),
                        black_box(&triangles),
                    );
                });
            },
        );

        // Report cycles per pixel
        #[cfg(target_arch = "x86_64")]
        {
            let mut fb_test = Framebuffer::new(1920, 1080).unwrap();
            let mut zb_test = ZBuffer::new(1920, 1080).unwrap();
            let mut renderer_test = TileRenderer::new(1920, 1080);

            let cycles = measure_cycles(|| {
                fb_test.clear(0xFF_00_00_00);
                zb_test.clear();
                renderer_test.render_batch(&mut fb_test, &mut zb_test, &triangles);
            });

            let pixels_drawn = (*scanline_len as usize) * 5 * triangles.len(); // ~5 pixels height per triangle
            let cycles_per_pixel = cycles as f64 / pixels_drawn as f64;
            eprintln!(
                "Scanline length {scanline_len}: {cycles} cycles/frame ({cycles_per_pixel:.2} cycles/pixel)"
            );
        }
    }

    group.finish();
}

/// Benchmark memory access patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");

    // Sequential writes (should hit fast path)
    group.bench_function("sequential_write", |b| {
        let mut fb = Framebuffer::new(1920, 1080).unwrap();
        b.iter(|| {
            fb.clear(black_box(0xFF_FF_FF_FF));
        });
    });

    // Random access pattern (tests cache misses)
    group.bench_function("random_access", |b| {
        let mut fb = Framebuffer::new(1920, 1080).unwrap();
        let mut zb = ZBuffer::new(1920, 1080).unwrap();

        // Scattered triangles across the framebuffer
        let triangles = generate_scattered_triangles(100, 1920, 1080);
        let mut renderer = TileRenderer::new(1920, 1080);

        b.iter(|| {
            fb.clear(black_box(0xFF_00_00_00));
            zb.clear();
            renderer.render_batch(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(&triangles),
            );
        });
    });

    group.finish();
}

/// Benchmark tile size impact on cache locality
fn bench_tile_locality(c: &mut Criterion) {
    let mut group = c.benchmark_group("tile_locality");

    // Scene that fits in L1 cache (32×32 tiles)
    group.bench_function("small_scene_tiled", |b| {
        let mut fb = Framebuffer::new(1920, 1080).unwrap();
        let mut zb = ZBuffer::new(1920, 1080).unwrap();
        let mut renderer = TileRenderer::new(1920, 1080);
        let triangles = generate_test_scene(10, 1920, 1080);

        b.iter(|| {
            fb.clear(black_box(0xFF_00_00_00));
            zb.clear();
            renderer.render_batch(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(&triangles),
            );
        });
    });

    // Large scene that spills cache
    group.bench_function("large_scene_tiled", |b| {
        let mut fb = Framebuffer::new(3840, 2160).unwrap();
        let mut zb = ZBuffer::new(3840, 2160).unwrap();
        let mut renderer = TileRenderer::new(3840, 2160);
        let triangles = generate_test_scene(1000, 3840, 2160);

        b.iter(|| {
            fb.clear(black_box(0xFF_00_00_00));
            zb.clear();
            renderer.render_batch(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(&triangles),
            );
        });
    });

    group.finish();
}

/// Generate horizontal triangles of specific scanline length
fn generate_horizontal_triangles(scanline_len: u32, count: usize) -> Vec<ClipTriangle> {
    let mut triangles = Vec::with_capacity(count);
    let width = scanline_len as f32;

    for i in 0..count {
        let y = (i as f32) * 10.0;
        let x = 100.0;
        let depth = 5.0 + (i % 5) as f32;

        // Horizontal triangles with specific width
        let v0 = (Vec3::new(x, y, depth), 1.0);
        let v1 = (Vec3::new(x + width, y, depth), 1.0);
        let v2 = (Vec3::new(x + width / 2.0, y + 5.0, depth), 1.0);

        let color = 0xFF_FF_00_00;
        triangles.push((v0, v1, v2, color));
    }

    triangles
}

/// Generate scattered triangles across the framebuffer
fn generate_scattered_triangles(count: usize, width: u32, height: u32) -> Vec<ClipTriangle> {
    let mut triangles = Vec::with_capacity(count);

    for i in 0..count {
        // Pseudo-random scattering using prime number modulo
        let x = ((i * 137) % (width as usize)) as f32;
        let y = ((i * 199) % (height as usize)) as f32;
        let depth = 5.0 + ((i * 73) % 10) as f32;
        let size = 50.0;

        let v0 = (Vec3::new(x, y, depth), 1.0);
        let v1 = (Vec3::new(x + size, y, depth + 0.1), 1.0);
        let v2 = (Vec3::new(x + size / 2.0, y + size, depth + 0.2), 1.0);

        let color = 0xFF_00_00_00 | ((i as u32) << 8);
        triangles.push((v0, v1, v2, color));
    }

    triangles
}

/// Generate test scene with triangles
fn generate_test_scene(count: usize, width: u32, height: u32) -> Vec<ClipTriangle> {
    let mut triangles = Vec::with_capacity(count);

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

criterion_group!(
    benches,
    bench_hiz_reduction_cycles,
    bench_scanline_rasterization_cycles,
    bench_memory_patterns,
    bench_tile_locality,
);
criterion_main!(benches);
