use abrash::{framebuffer::Framebuffer, math::Vec3, rasterizer::TileRenderer, zbuffer::ZBuffer};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_scanline_rasterization(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;

    // Create a single large triangle that covers most of the screen
    // This maximizes the time spent in the rasterization loop vs setup/binning
    let v0 = (Vec3::new(-0.9, 0.9, 5.0), 1.0);
    let v1 = (Vec3::new(0.9, 0.9, 5.0), 1.0);
    let v2 = (Vec3::new(0.0, -0.9, 5.0), 1.0);
    let color = 0xFFFF_0000;

    let triangles = vec![(v0, v1, v2, color)];

    let mut group = c.benchmark_group("scanline_micro");

    group.bench_function("large_triangle_1080p", |b| {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut renderer = TileRenderer::new(width, height);

        b.iter(|| {
            fb.clear(black_box(0xFF_00_00_00));
            zb.clear();
            renderer.render_batch(&mut fb, &mut zb, black_box(&triangles));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_scanline_rasterization);
criterion_main!(benches);
