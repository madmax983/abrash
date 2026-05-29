use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec3;
use abrash_render::rasterizer::line::draw_line_3d;
use abrash_render::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_draw_line_3d(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();

    let v0 = (Vec3::new(-0.8, -0.8, 0.5), 1.0);
    let v1 = (Vec3::new(0.8, 0.8, 0.5), 1.0);

    c.bench_function("draw_line_3d", |b| {
        b.iter(|| {
            draw_line_3d(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(v0),
                black_box(v1),
                black_box(0xFFFF_FFFF),
            );
        });
    });
}

criterion_group!(benches, bench_draw_line_3d);
criterion_main!(benches);
