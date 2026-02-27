use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::{ClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_overdraw_sorting(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);

    // Create 500 overlapping triangles at the center of the screen
    // Each triangle covers the center tile (and others)
    // Z varies to force sorting
    let mut triangles: Vec<ClipTriangle> = Vec::with_capacity(500);
    for i in 0..500 {
        let z = 5.0 + (i as f32 * 0.01); // 5.0 .. 10.0
        // Front-to-back or back-to-front doesn't matter for the *sort* time itself,
        // but random might be more realistic. Let's do reverse order (back to front)
        // so sorting has to do work if it wants front-to-back.
        let v0 = (Vec3::new(-0.5, 0.5, z), z);
        let v1 = (Vec3::new(0.5, 0.5, z), z);
        let v2 = (Vec3::new(0.0, -0.5, z), z);
        triangles.push((v0, v1, v2, 0xFFFFFFFF));
    }

    // Benchmark the full render pipeline, which includes sorting
    c.bench_function("tile_render_overdraw_500", |b| {
        b.iter(|| {
            fb.clear(0xFF000000);
            zb.clear();
            renderer.render_batch(&mut fb, &mut zb, &triangles);
        })
    });
}

criterion_group!(benches, bench_overdraw_sorting);
criterion_main!(benches);
