//! Performance benchmarks for software two-level hierarchical binning.
//!
//! Compares single-level CPU binning vs two-level (coarse + Hi-Z + fine) CPU binning.

use abrash::{framebuffer::Framebuffer, math::Vec3, rasterizer::TileRenderer, zbuffer::ZBuffer};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::time::Duration;

type ClipTriangle = ((Vec3, f32), (Vec3, f32), (Vec3, f32), u32);

/// Generate a grid of triangles at a specific depth
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

/// Generate a single large triangle covering the screen
fn generate_large_triangle(depth: f32) -> Vec<ClipTriangle> {
    vec![(
        (Vec3::new(-2.0, -2.0, depth), 1.0),
        (Vec3::new(2.0, -2.0, depth), 1.0),
        (Vec3::new(0.0, 2.0, depth), 1.0),
        0xFF0000FF,
    )]
}

/// Generate overlapping layers of triangles at different depths
fn generate_layered_scene(triangles_per_layer: usize, layer_count: usize) -> Vec<ClipTriangle> {
    let mut triangles = Vec::with_capacity(triangles_per_layer * layer_count);
    for layer in 0..layer_count {
        let depth = 0.5 + (layer as f32 * 0.1);
        triangles.extend(generate_triangle_grid(triangles_per_layer, depth));
    }
    triangles
}

fn bench_software_binning_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("software_binning");
    group.measurement_time(Duration::from_secs(5));

    // Case 1: Large screen-covering triangle with partial occlusion
    group.throughput(Throughput::Elements(1));

    // Helper to setup checkerboard occlusion
    let setup_occluded_scene = |width: u32, height: u32| {
        let fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let slice = zb.as_mut_slice();
        for y in 0..height {
            for x in 0..width {
                let cx = x / 128;
                let cy = y / 128;
                if (cx + cy) % 2 == 0 {
                    slice[(y * width + x) as usize] = 0.1; // Occluder
                } else {
                    slice[(y * width + x) as usize] = 1.0; // Far
                }
            }
        }
        (fb, zb)
    };

    group.bench_function("single_level_large_occluded", |b| {
        let width = 1920;
        let height = 1080;
        let mut renderer = TileRenderer::new(width, height);
        renderer.enable_hiz();
        let (mut fb, mut zb) = setup_occluded_scene(width, height);
        let triangle = generate_large_triangle(0.5);

        b.iter(|| {
            renderer.render_batch(black_box(&mut fb), black_box(&mut zb), black_box(&triangle));
        });
    });

    group.bench_function("two_level_large_occluded", |b| {
        let width = 1920;
        let height = 1080;
        let mut renderer = TileRenderer::new(width, height);
        renderer.enable_software_two_level_binning();
        let (mut fb, mut zb) = setup_occluded_scene(width, height);
        let triangle = generate_large_triangle(0.5);

        b.iter(|| {
            renderer.render_batch(black_box(&mut fb), black_box(&mut zb), black_box(&triangle));
        });
    });

    // Case 2: Layered scene (standard test)
    let width = 1920;
    let height = 1080;
    let triangles_per_layer = 100;

    {
        let &layer_count = &10;
        let total_tris = triangles_per_layer * layer_count;
        group.throughput(Throughput::Elements(total_tris as u64));

        group.bench_with_input(
            BenchmarkId::new("single_level_layered", total_tris),
            &layer_count,
            |b, &layers| {
                let mut renderer = TileRenderer::new(width, height);
                renderer.enable_hiz();
                let mut fb = Framebuffer::new(width, height).unwrap();
                let mut zb = ZBuffer::new(width, height).unwrap();
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

        group.bench_with_input(
            BenchmarkId::new("two_level_layered", total_tris),
            &layer_count,
            |b, &layers| {
                let mut renderer = TileRenderer::new(width, height);
                renderer.enable_software_two_level_binning();
                let mut fb = Framebuffer::new(width, height).unwrap();
                let mut zb = ZBuffer::new(width, height).unwrap();
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

criterion_group!(benches, bench_software_binning_comparison);
criterion_main!(benches);
