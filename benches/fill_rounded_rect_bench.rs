use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::rect::fill_rounded_rect;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_fill_rounded_rect(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let color = 0xFFFF_FFFF;

    c.bench_function("fill_rounded_rect_100x100_r10", |b| {
        b.iter(|| {
            fill_rounded_rect(
                &mut fb,
                black_box(100),
                black_box(100),
                black_box(100),
                black_box(100),
                black_box(10),
                black_box(color),
            );
        });
    });

    c.bench_function("fill_rounded_rect_400x300_r50", |b| {
        b.iter(|| {
            fill_rounded_rect(
                &mut fb,
                black_box(100),
                black_box(100),
                black_box(400),
                black_box(300),
                black_box(50),
                black_box(color),
            );
        });
    });
}

criterion_group!(benches, bench_fill_rounded_rect);
criterion_main!(benches);
