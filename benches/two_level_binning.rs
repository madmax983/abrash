//! Performance benchmarks for two-level hierarchical GPU binning.
//!
//! Compares single-level GPU binning vs two-level (coarse + Hi-Z + fine) binning
//! to measure the performance impact of hierarchical culling.

#![cfg(all(feature = "backend-win32", feature = "gpu-binning"))]

use abrash::{framebuffer::Framebuffer, math::Vec3, tile_renderer::TileRenderer, zbuffer::ZBuffer};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::time::Duration;

type ClipTriangle = ((Vec3, f32), (Vec3, f32), (Vec3, f32), u32);

/// Generate a grid of triangles at a specific depth for benchmarking
fn generate_triangle_grid(count: usize, depth: f32) -> Vec<ClipTriangle> {
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

            triangles.push((
                (Vec3::new(x - size, y - size, depth), 1.0),
                (Vec3::new(x + size, y - size, depth), 1.0),
                (Vec3::new(x, y + size, depth), 1.0),
                0xFF0000FF,
            ));
        }
    }

    triangles
}

/// Generate overlapping layers of triangles at different depths
/// to test Hi-Z culling effectiveness
fn generate_layered_scene(triangles_per_layer: usize, layer_count: usize) -> Vec<ClipTriangle> {
    let mut triangles = Vec::new();

    // Front-to-back layers (optimal for Hi-Z)
    for layer in 0..layer_count {
        let depth = 0.5 + (layer as f32 * 0.1);
        triangles.extend(generate_triangle_grid(triangles_per_layer, depth));
    }

    triangles
}

/// Benchmark single-level GPU binning (baseline)
fn bench_single_level_binning(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_level_binning");
    group.measurement_time(Duration::from_secs(10));

    for &(width, height) in &[(1920, 1080), (3840, 2160)] {
        for &tri_count in &[10, 100, 500, 1000] {
            group.throughput(Throughput::Elements(tri_count as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("{}x{}", width, height), tri_count),
                &(width, height, tri_count),
                |b, &(w, h, count)| {
                    let mut renderer = TileRenderer::new(w, h);

                    if renderer.enable_gpu_binning().is_err() {
                        eprintln!("GPU binning not available, skipping");
                        return;
                    }

                    let mut fb = Framebuffer::new(w, h).unwrap();
                    let mut zb = ZBuffer::new(w, h).unwrap();
                    let triangles = generate_triangle_grid(count, 0.5);

                    b.iter(|| {
                        renderer.render_batch(
                            black_box(&mut fb),
                            black_box(&mut zb),
                            black_box(&triangles),
                        );
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark two-level GPU binning with Hi-Z culling
fn bench_two_level_binning(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_level_binning");
    group.measurement_time(Duration::from_secs(10));

    for &(width, height) in &[(1920, 1080), (3840, 2160)] {
        for &tri_count in &[10, 100, 500, 1000] {
            group.throughput(Throughput::Elements(tri_count as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("{}x{}", width, height), tri_count),
                &(width, height, tri_count),
                |b, &(w, h, count)| {
                    let mut renderer = TileRenderer::new(w, h);

                    if renderer.enable_gpu_binning().is_err() {
                        eprintln!("GPU binning not available, skipping");
                        return;
                    }

                    // Enable Hi-Z for coarse bin culling
                    renderer.enable_hiz();

                    let mut fb = Framebuffer::new(w, h).unwrap();
                    let mut zb = ZBuffer::new(w, h).unwrap();
                    let triangles = generate_triangle_grid(count, 0.5);

                    b.iter(|| {
                        renderer.render_batch(
                            black_box(&mut fb),
                            black_box(&mut zb),
                            black_box(&triangles),
                        );
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark two-level binning with layered scene (high occlusion)
fn bench_two_level_layered(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_level_layered");
    group.measurement_time(Duration::from_secs(10));

    for &(width, height) in &[(1920, 1080), (3840, 2160)] {
        for &layer_count in &[2, 5, 10] {
            let triangles_per_layer = 100;
            let total_tris = triangles_per_layer * layer_count;

            group.throughput(Throughput::Elements(total_tris as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("{}x{}", width, height), total_tris),
                &(width, height, layer_count),
                |b, &(w, h, layers)| {
                    let mut renderer = TileRenderer::new(w, h);

                    if renderer.enable_gpu_binning().is_err() {
                        eprintln!("GPU binning not available, skipping");
                        return;
                    }

                    renderer.enable_hiz();

                    let mut fb = Framebuffer::new(w, h).unwrap();
                    let mut zb = ZBuffer::new(w, h).unwrap();
                    let triangles = generate_layered_scene(triangles_per_layer, layers);

                    b.iter(|| {
                        renderer.render_batch(
                            black_box(&mut fb),
                            black_box(&mut zb),
                            black_box(&triangles),
                        );
                    });
                },
            );
        }
    }

    group.finish();
}

/// Compare frame time: single-level vs two-level binning
fn bench_frame_time_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_time_comparison");
    group.measurement_time(Duration::from_secs(10));

    for &tri_count in &[100, 500, 1000] {
        // Single-level binning
        group.bench_with_input(
            BenchmarkId::new("single_level", tri_count),
            &tri_count,
            |b, &count| {
                let mut renderer = TileRenderer::new(1920, 1080);

                if renderer.enable_gpu_binning().is_err() {
                    eprintln!("GPU binning not available");
                    return;
                }

                let mut fb = Framebuffer::new(1920, 1080).unwrap();
                let mut zb = ZBuffer::new(1920, 1080).unwrap();
                let triangles = generate_triangle_grid(count, 0.5);

                b.iter(|| {
                    renderer.render_batch(
                        black_box(&mut fb),
                        black_box(&mut zb),
                        black_box(&triangles),
                    );
                });
            },
        );

        // Two-level binning
        group.bench_with_input(
            BenchmarkId::new("two_level", tri_count),
            &tri_count,
            |b, &count| {
                let mut renderer = TileRenderer::new(1920, 1080);

                if renderer.enable_gpu_binning().is_err() {
                    eprintln!("GPU binning not available");
                    return;
                }

                renderer.enable_hiz();

                let mut fb = Framebuffer::new(1920, 1080).unwrap();
                let mut zb = ZBuffer::new(1920, 1080).unwrap();
                let triangles = generate_triangle_grid(count, 0.5);

                b.iter(|| {
                    renderer.render_batch(
                        black_box(&mut fb),
                        black_box(&mut zb),
                        black_box(&triangles),
                    );
                });
            },
        );
    }

    group.finish();
}

/// Benchmark layered scene comparison (where two-level should excel)
fn bench_layered_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("layered_comparison");
    group.measurement_time(Duration::from_secs(10));

    let triangles_per_layer = 100;
    for &layer_count in &[2, 5, 10] {
        let total_tris = triangles_per_layer * layer_count;

        // Single-level binning (no culling)
        group.bench_with_input(
            BenchmarkId::new("single_level", total_tris),
            &layer_count,
            |b, &layers| {
                let mut renderer = TileRenderer::new(1920, 1080);

                if renderer.enable_gpu_binning().is_err() {
                    eprintln!("GPU binning not available");
                    return;
                }

                let mut fb = Framebuffer::new(1920, 1080).unwrap();
                let mut zb = ZBuffer::new(1920, 1080).unwrap();
                let triangles = generate_layered_scene(triangles_per_layer, layers);

                b.iter(|| {
                    renderer.render_batch(
                        black_box(&mut fb),
                        black_box(&mut zb),
                        black_box(&triangles),
                    );
                });
            },
        );

        // Two-level binning (with Hi-Z culling)
        group.bench_with_input(
            BenchmarkId::new("two_level", total_tris),
            &layer_count,
            |b, &layers| {
                let mut renderer = TileRenderer::new(1920, 1080);

                if renderer.enable_gpu_binning().is_err() {
                    eprintln!("GPU binning not available");
                    return;
                }

                renderer.enable_hiz();

                let mut fb = Framebuffer::new(1920, 1080).unwrap();
                let mut zb = ZBuffer::new(1920, 1080).unwrap();
                let triangles = generate_layered_scene(triangles_per_layer, layers);

                b.iter(|| {
                    renderer.render_batch(
                        black_box(&mut fb),
                        black_box(&mut zb),
                        black_box(&triangles),
                    );
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_level_binning,
    bench_two_level_binning,
    bench_two_level_layered,
    bench_frame_time_comparison,
    bench_layered_comparison
);
criterion_main!(benches);
