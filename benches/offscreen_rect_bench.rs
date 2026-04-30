use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::rect::fill_rounded_rect;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_fill_rounded_rect_offscreen(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();

    c.bench_function("fill_rounded_rect_massive_offscreen", |b| {
        b.iter(|| {
            fill_rounded_rect(
                black_box(&mut fb),
                black_box(-2147483640),
                black_box(-2147483640),
                black_box(2147483000),
                black_box(2147483000),
                black_box(1073741000),
                black_box(0xFFFFFFFF),
            )
        })
    });
}

criterion_group!(benches, bench_fill_rounded_rect_offscreen);
criterion_main!(benches);
