use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::pbr::fill_triangle_pbr;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_fill_triangle_pbr_large(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Large triangle covering most of the screen
    let v0 = (
        (Vec3::new(0.0, 500.0, 10.0), 10.0), // W=10 for perspective
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 10.0, 0.0),
    );
    let v1 = (
        (Vec3::new(-800.0, -500.0, 10.0), 10.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(-10.0, -10.0, 0.0),
    );
    let v2 = (
        (Vec3::new(800.0, -500.0, 10.0), 10.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(10.0, -10.0, 0.0),
    );

    let albedo = Vec3::new(1.0, 0.0, 0.0);
    let metallic = 0.5;
    let roughness = 0.5;
    let ao = 1.0;
    let light_dir = Vec3::new(0.0, 0.0, -1.0).normalize();
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let view_pos = Vec3::new(0.0, 0.0, 5.0);

    let mut group = c.benchmark_group("pbr");
    group.sample_size(10);

    group.bench_function("fill_triangle_pbr_1080p", |b| {
        b.iter(|| {
            // Clear Z-Buffer roughly (just setting one value isn't enough for full clear but we want to benchmark drawing)
            // Actually, fill_triangle respects Z-buffer. If Z is closer, it won't draw.
            // We should clear it or ensure Z is always passing.
            // Since we re-draw the same triangle, Z will be equal. PBR typically uses <= or <.
            // If we use <, subsequent draws fail.
            // Let's clear Z buffer every time or use a fresh one. Allocating fresh one is slow.
            // Clearing is O(N).
            // Better: Move Z far away.
            // Actually, just let it overdraw. If it uses `<` it will fail.
            // Let's verify implementation: `if z < *depth_val`.
            // So subsequent calls will fail.
            // I must clear Z buffer.
            zb.clear();

            fill_triangle_pbr(
                &mut fb,
                &mut zb,
                v0,
                v1,
                v2,
                albedo,
                metallic,
                roughness,
                ao,
                light_dir,
                light_color,
                view_pos,
            );
        });
    });
}

criterion_group!(benches, bench_fill_triangle_pbr_large);
criterion_main!(benches);
