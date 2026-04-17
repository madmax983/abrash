use criterion::{Criterion, black_box, criterion_group, criterion_main};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::{draw_rect, fill_rect, draw_rounded_rect, fill_rounded_rect};

fn bench_rects(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let color = 0xFFFFFFFF;

    c.bench_function("draw_rect_100", |b| {
        b.iter(|| draw_rect(&mut fb, black_box(100), black_box(100), black_box(200), black_box(200), black_box(color)))
    });

    c.bench_function("fill_rect_100", |b| {
        b.iter(|| fill_rect(&mut fb, black_box(100), black_box(100), black_box(200), black_box(200), black_box(color)))
    });

    c.bench_function("draw_rounded_rect_100", |b| {
        b.iter(|| draw_rounded_rect(&mut fb, black_box(100), black_box(100), black_box(200), black_box(200), black_box(20), black_box(color)))
    });

    c.bench_function("fill_rounded_rect_100", |b| {
        b.iter(|| fill_rounded_rect(&mut fb, black_box(100), black_box(100), black_box(200), black_box(200), black_box(20), black_box(color)))
    });
}

criterion_group!(benches, bench_rects);
criterion_main!(benches);
