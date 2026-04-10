use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::{draw_circle, fill_circle, draw_circle_aa};

fn draw_circle_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let color = 0xFFFFFFFF;

    c.bench_function("draw_circle_r100", |b| b.iter(|| {
        draw_circle(black_box(&mut fb), black_box(400), black_box(300), black_box(100), black_box(color));
    }));

    c.bench_function("fill_circle_r100", |b| b.iter(|| {
        fill_circle(black_box(&mut fb), black_box(400), black_box(300), black_box(100), black_box(color));
    }));

    c.bench_function("draw_circle_aa_r100", |b| b.iter(|| {
        draw_circle_aa(black_box(&mut fb), black_box(400), black_box(300), black_box(100), black_box(color));
    }));
}

criterion_group!(benches, draw_circle_benchmark);
criterion_main!(benches);
