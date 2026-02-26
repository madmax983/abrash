use abrash::{framebuffer::Framebuffer, math::Vec3, rasterizer::TileRenderer, zbuffer::ZBuffer};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_small_triangles(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);

    // Create a grid of small triangles
    // Each triangle is approx 4x4 pixels
    let mut triangles = Vec::new();
    let rows = 100;
    let cols = 100;

    // Grid covers 0..1 in NDC
    let step_x = 1.0 / cols as f32;
    let step_y = 1.0 / rows as f32;

    for y in 0..rows {
        for x in 0..cols {
            let x0 = (x as f32) * step_x * 2.0 - 1.0;
            let y0 = (y as f32) * step_y * 2.0 - 1.0;
            let x1 = x0 + step_x;
            let y1 = y0 + step_y;
            let z = 5.0;

            // Two triangles for a quad, but let's just do one to keep it simple and small
            let v0 = (Vec3::new(x0, y0, z), 1.0);
            let v1 = (Vec3::new(x1, y0, z), 1.0);
            let v2 = (Vec3::new(x0, y1, z), 1.0);
            let color = 0xFFFF_0000; // Red

            triangles.push((v0, v1, v2, color));
        }
    }

    let mut group = c.benchmark_group("tile_rendering_small_tris");
    group.sample_size(50); // Lower sample size for faster benches

    group.bench_function("10k_small_triangles", |b| {
        b.iter(|| {
            fb.clear(black_box(0xFF_00_00_00));
            zb.clear();
            renderer.render_batch(&mut fb, &mut zb, black_box(&triangles));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_small_triangles);
criterion_main!(benches);
