use abrash::{framebuffer::Framebuffer, math::Vec3, rasterizer::TileRenderer, zbuffer::ZBuffer};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_z_cull(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;

    // Triangle at z=5.0
    let v0 = (Vec3::new(-0.9, 0.9, 5.0), 1.0);
    let v1 = (Vec3::new(0.9, 0.9, 5.0), 1.0);
    let v2 = (Vec3::new(0.0, -0.9, 5.0), 1.0);
    let color = 0xFFFF_0000;

    let triangles = vec![(v0, v1, v2, color)];

    let mut group = c.benchmark_group("z_cull");

    group.bench_function("fully_occluded", |b| {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut renderer = TileRenderer::new(width, height);

        // Pre-fill Z-buffer with 1.0 (closer than triangle at 5.0)
        // Since we re-use zb in the loop, we must reset it every iteration.
        // However, `renderer.render_batch` doesn't clear zb. We can manually fill it.

        b.iter(|| {
            // Fill Z-buffer with 1.0 (occluder)
            zb.clear_val(black_box(1.0));
            // Framebuffer doesn't matter for performance much here, but let's clear it
            fb.clear(0xFF_00_00_00);

            // Render triangle at 5.0. Should fail Z-test everywhere.
            renderer.render_batch(&mut fb, &mut zb, black_box(&triangles));
        });
    });

    group.bench_function("fully_visible", |b| {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut renderer = TileRenderer::new(width, height);

        b.iter(|| {
            // Fill Z-buffer with 10.0 (farther than triangle at 5.0)
            zb.clear_val(black_box(10.0));
            fb.clear(0xFF_00_00_00);

            // Render triangle at 5.0. Should pass Z-test everywhere.
            renderer.render_batch(&mut fb, &mut zb, black_box(&triangles));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_z_cull);
criterion_main!(benches);
