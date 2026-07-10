use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::draw_line_2d;

fn bench_draw_line_2d(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut group = c.benchmark_group("draw_line_2d");

    group.bench_function("diagonal_line", |b| {
        b.iter(|| {
            draw_line_2d(
                black_box(&mut fb),
                black_box(10), black_box(10),
                black_box(790), black_box(590),
                black_box(0xFFFF_FFFF)
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_draw_line_2d);
criterion_main!(benches);
