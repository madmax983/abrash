//! GPU Hi-Z Pyramid Build Benchmarks
//!
//! Measures GPU vs CPU pyramid build performance across different resolutions.
//! Follows Abrash's "measure don't guess" principle.
//!
//! # Benchmark Coverage
//! - CPU pyramid build (1080p, 4K)
//! - GPU pyramid build (1080p, 4K)
//! - GPU upload overhead
//! - GPU download overhead
//! - End-to-end (upload + build + download)

#![cfg(all(feature = "backend-win32", feature = "gpu-binning"))]

use abrash::{hiz_buffer::HiZBuffer, zbuffer::ZBuffer};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

/// Resolution configurations for benchmarking
struct Resolution {
    name: &'static str,
    width: u32,
    height: u32,
}

const RESOLUTIONS: &[Resolution] = &[
    Resolution {
        name: "1080p",
        width: 1920,
        height: 1080,
    },
    Resolution {
        name: "4K",
        width: 3840,
        height: 2160,
    },
];

/// Setup zbuffer with pseudo-random pattern
fn setup_zbuffer(width: u32, height: u32) -> ZBuffer {
    let mut zb = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let slice = zb.as_mut_slice();

    // Fill with pseudo-random pattern to simulate realistic scene
    for i in 0..slice.len() {
        let hash = ((i.wrapping_mul(2654435761)) >> 16) as f32 / 65536.0;
        slice[i] = hash * 100.0;
    }

    zb
}

/// Benchmark CPU pyramid build across resolutions
fn bench_cpu_pyramid_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_pyramid_build");

    for res in RESOLUTIONS {
        let zb = setup_zbuffer(res.width, res.height);

        group.bench_with_input(BenchmarkId::from_parameter(res.name), &zb, |b, zb| {
            let mut hiz = HiZBuffer::new(res.width, res.height);

            b.iter(|| {
                hiz.build_pyramid(black_box(zb));
            });
        });
    }

    group.finish();
}

/// Benchmark GPU pyramid build across resolutions
fn bench_gpu_pyramid_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_pyramid_build");

    for res in RESOLUTIONS {
        let zb = setup_zbuffer(res.width, res.height);

        group.bench_with_input(BenchmarkId::from_parameter(res.name), &zb, |b, zb| {
            let mut hiz = HiZBuffer::new(res.width, res.height);
            hiz.enable_gpu_build()
                .expect("Failed to enable GPU pyramid build");

            b.iter(|| {
                hiz.build_pyramid(black_box(zb));
            });
        });
    }

    group.finish();
}

/// Benchmark GPU upload overhead (zbuffer upload to GPU)
/// This isolates the upload cost from the compute cost
fn bench_gpu_upload_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_upload_overhead");

    for res in RESOLUTIONS {
        let zb = setup_zbuffer(res.width, res.height);

        group.bench_with_input(BenchmarkId::from_parameter(res.name), &zb, |b, zb| {
            let mut hiz = HiZBuffer::new(res.width, res.height);
            hiz.enable_gpu_build()
                .expect("Failed to enable GPU pyramid build");

            b.iter(|| {
                // Measure just the upload (first build includes upload)
                // Subsequent builds reuse uploaded data
                hiz.invalidate();
                hiz.build_pyramid(black_box(zb));
            });
        });
    }

    group.finish();
}

/// Benchmark GPU download overhead (pyramid download from GPU)
/// This isolates the download cost (if any) from the compute cost
fn bench_gpu_download_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_download_overhead");

    for res in RESOLUTIONS {
        let zb = setup_zbuffer(res.width, res.height);

        group.bench_with_input(BenchmarkId::from_parameter(res.name), &zb, |b, zb| {
            let mut hiz = HiZBuffer::new(res.width, res.height);
            hiz.enable_gpu_build()
                .expect("Failed to enable GPU pyramid build");

            // Pre-build pyramid
            hiz.build_pyramid(zb);

            b.iter(|| {
                // Measure occlusion query (forces download if not already done)
                // This is a proxy for download overhead
                hiz.is_potentially_visible(black_box(abrash::hiz_buffer::AABB3D {
                    min_x: 100,
                    max_x: 200,
                    min_y: 100,
                    max_y: 200,
                    min_depth: 5.0,
                    max_depth: 10.0,
                }));
            });
        });
    }

    group.finish();
}

/// Benchmark end-to-end GPU pipeline (upload + build + download)
/// This measures the complete cost including all GPU transfers
fn bench_gpu_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_end_to_end");

    for res in RESOLUTIONS {
        let zb = setup_zbuffer(res.width, res.height);

        group.bench_with_input(BenchmarkId::from_parameter(res.name), &zb, |b, zb| {
            let mut hiz = HiZBuffer::new(res.width, res.height);
            hiz.enable_gpu_build()
                .expect("Failed to enable GPU pyramid build");

            b.iter(|| {
                // Measure complete pipeline
                hiz.invalidate();
                hiz.build_pyramid(black_box(zb));

                // Force download via occlusion query
                hiz.is_potentially_visible(black_box(abrash::hiz_buffer::AABB3D {
                    min_x: 100,
                    max_x: 200,
                    min_y: 100,
                    max_y: 200,
                    min_depth: 5.0,
                    max_depth: 10.0,
                }));
            });
        });
    }

    group.finish();
}

/// Benchmark CPU vs GPU speedup comparison
/// This benchmark runs both CPU and GPU in the same group for direct comparison
fn bench_cpu_vs_gpu_comparison(c: &mut Criterion) {
    for res in RESOLUTIONS {
        let mut group = c.benchmark_group(format!("cpu_vs_gpu_{}", res.name));

        let zb = setup_zbuffer(res.width, res.height);

        // CPU baseline
        group.bench_function("cpu", |b| {
            let mut hiz = HiZBuffer::new(res.width, res.height);

            b.iter(|| {
                hiz.build_pyramid(black_box(&zb));
            });
        });

        // GPU comparison
        group.bench_function("gpu", |b| {
            let mut hiz = HiZBuffer::new(res.width, res.height);
            hiz.enable_gpu_build()
                .expect("Failed to enable GPU pyramid build");

            b.iter(|| {
                hiz.build_pyramid(black_box(&zb));
            });
        });

        group.finish();
    }
}

/// Benchmark pyramid build with varying zbuffer patterns
/// Tests performance across different depth distributions
fn bench_gpu_varying_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_varying_patterns");

    let width = 1920;
    let height = 1080;

    // Pattern 1: Uniform depth
    let mut zb_uniform = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    {
        let slice = zb_uniform.as_mut_slice();
        for i in 0..slice.len() {
            slice[i] = 5.0;
        }
    }

    group.bench_function("uniform", |b| {
        let mut hiz = HiZBuffer::new(width, height).unwrap();
        hiz.enable_gpu_build()
            .expect("Failed to enable GPU pyramid build");

        b.iter(|| {
            hiz.build_pyramid(black_box(&zb_uniform));
        });
    });

    // Pattern 2: Gradient
    let mut zb_gradient = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    {
        let slice = zb_gradient.as_mut_slice();
        for y in 0..height {
            for x in 0..width {
                slice[(y * width + x) as usize] = (x + y) as f32 * 0.01;
            }
        }
    }

    group.bench_function("gradient", |b| {
        let mut hiz = HiZBuffer::new(width, height).unwrap();
        hiz.enable_gpu_build()
            .expect("Failed to enable GPU pyramid build");

        b.iter(|| {
            hiz.build_pyramid(black_box(&zb_gradient));
        });
    });

    // Pattern 3: Checkerboard
    let mut zb_checkerboard = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    {
        let slice = zb_checkerboard.as_mut_slice();
        for y in 0..height {
            for x in 0..width {
                let is_even = ((x / 64) + (y / 64)) % 2 == 0;
                slice[(y * width + x) as usize] = if is_even { 3.0 } else { 8.0 };
            }
        }
    }

    group.bench_function("checkerboard", |b| {
        let mut hiz = HiZBuffer::new(width, height).unwrap();
        hiz.enable_gpu_build()
            .expect("Failed to enable GPU pyramid build");

        b.iter(|| {
            hiz.build_pyramid(black_box(&zb_checkerboard));
        });
    });

    // Pattern 4: Random (already tested in main benchmarks)
    let zb_random = setup_zbuffer(width, height);

    group.bench_function("random", |b| {
        let mut hiz = HiZBuffer::new(width, height).unwrap();
        hiz.enable_gpu_build()
            .expect("Failed to enable GPU pyramid build");

        b.iter(|| {
            hiz.build_pyramid(black_box(&zb_random));
        });
    });

    group.finish();
}

/// Benchmark GPU pyramid build with varying resolutions
/// Tests scaling characteristics from small to large
fn bench_gpu_resolution_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_resolution_scaling");

    let resolutions = vec![
        ("640x480", 640, 480),
        ("800x600", 800, 600),
        ("1280x720", 1280, 720),
        ("1920x1080", 1920, 1080),
        ("2560x1440", 2560, 1440),
        ("3840x2160", 3840, 2160),
    ];

    for (name, width, height) in resolutions {
        let zb = setup_zbuffer(width, height);

        group.bench_with_input(BenchmarkId::from_parameter(name), &zb, |b, zb| {
            let mut hiz = HiZBuffer::new(width, height).unwrap();
            hiz.enable_gpu_build()
                .expect("Failed to enable GPU pyramid build");

            b.iter(|| {
                hiz.build_pyramid(black_box(zb));
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_cpu_pyramid_build,
    bench_gpu_pyramid_build,
    bench_gpu_upload_overhead,
    bench_gpu_download_overhead,
    bench_gpu_end_to_end,
    bench_cpu_vs_gpu_comparison,
    bench_gpu_varying_patterns,
    bench_gpu_resolution_scaling,
);
criterion_main!(benches);
