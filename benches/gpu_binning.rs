//! Performance benchmarks for GPU compute binning
//!
//! Measures GPU vs CPU binning performance and overall frame time improvements.

#![cfg(all(feature = "backend-win32", feature = "gpu-binning"))]

use abrash::{framebuffer::Framebuffer, math::Vec3, rasterizer::tile::TileRenderer, zbuffer::ZBuffer};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

type ClipTriangle = ((Vec3, f32), (Vec3, f32), (Vec3, f32), u32);

/// Generate a grid of triangles for benchmarking
fn generate_triangle_grid(count: usize) -> Vec<ClipTriangle> {
    let mut triangles = Vec::with_capacity(count);
    let grid_size = (count as f32).sqrt() as usize;

    for i in 0..grid_size {
        for j in 0..grid_size {
            if triangles.len() >= count {
                break;
            }

            let x = (i as f32 / grid_size as f32) * 2.0 - 1.0;
            let y = (j as f32 / grid_size as f32) * 2.0 - 1.0;
            let size = 0.15;
            let z = 0.5;

            triangles.push((
                (Vec3::new(x - size, y - size, z), 1.0),
                (Vec3::new(x + size, y - size, z), 1.0),
                (Vec3::new(x, y + size, z), 1.0),
                0xFF0000FF,
            ));
        }
    }

    triangles
}

/// Benchmark CPU binning only
fn bench_cpu_binning(c: &mut Criterion) {
    let mut group = c.benchmark_group("binning_cpu");

    for count in [10, 50, 100, 200, 500, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut renderer = TileRenderer::new(1920, 1080);
            let mut fb = Framebuffer::new(1920, 1080).unwrap();
            let mut zb = ZBuffer::new(1920, 1080).unwrap();
            let triangles = generate_triangle_grid(count);

            b.iter(|| {
                renderer.render_batch(
                    black_box(&mut fb),
                    black_box(&mut zb),
                    black_box(&triangles),
                );
            });
        });
    }

    group.finish();
}

/// Benchmark GPU binning
fn bench_gpu_binning(c: &mut Criterion) {
    let mut group = c.benchmark_group("binning_gpu");

    for count in [10, 50, 100, 200, 500, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let mut renderer = TileRenderer::new(1920, 1080);

            // Enable GPU binning
            if renderer.enable_gpu_binning().is_err() {
                eprintln!("GPU binning not available, skipping benchmark");
                return;
            }

            let mut fb = Framebuffer::new(1920, 1080).unwrap();
            let mut zb = ZBuffer::new(1920, 1080).unwrap();
            let triangles = generate_triangle_grid(count);

            b.iter(|| {
                renderer.render_batch(
                    black_box(&mut fb),
                    black_box(&mut zb),
                    black_box(&triangles),
                );
            });
        });
    }

    group.finish();
}

/// Benchmark end-to-end frame time comparison
fn bench_frame_time_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_time");

    for count in [100, 500, 1000] {
        // CPU version
        group.bench_with_input(BenchmarkId::new("cpu", count), &count, |b, &count| {
            let mut renderer = TileRenderer::new(1920, 1080);
            let mut fb = Framebuffer::new(1920, 1080).unwrap();
            let mut zb = ZBuffer::new(1920, 1080).unwrap();
            let triangles = generate_triangle_grid(count);

            b.iter(|| {
                renderer.render_batch(
                    black_box(&mut fb),
                    black_box(&mut zb),
                    black_box(&triangles),
                );
            });
        });

        // GPU version
        group.bench_with_input(BenchmarkId::new("gpu", count), &count, |b, &count| {
            let mut renderer = TileRenderer::new(1920, 1080);

            if renderer.enable_gpu_binning().is_err() {
                eprintln!("GPU binning not available");
                return;
            }

            let mut fb = Framebuffer::new(1920, 1080).unwrap();
            let mut zb = ZBuffer::new(1920, 1080).unwrap();
            let triangles = generate_triangle_grid(count);

            b.iter(|| {
                renderer.render_batch(
                    black_box(&mut fb),
                    black_box(&mut zb),
                    black_box(&triangles),
                );
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_cpu_binning,
    bench_gpu_binning,
    bench_frame_time_comparison
);
criterion_main!(benches);
