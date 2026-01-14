use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash::framebuffer::Framebuffer;
use abrash::primitives::plot_pixel;

fn bench_clear(c: &mut Criterion) {
    c.bench_function("framebuffer_clear_800x600", |b| {
        let mut fb = Framebuffer::new(800, 600);
        b.iter(|| {
            fb.clear(black_box(0xFFFF0000));
        });
    });
}

fn bench_plot_pixel(c: &mut Criterion) {
    c.bench_function("plot_pixel", |b| {
        let mut fb = Framebuffer::new(800, 600);
        b.iter(|| {
            plot_pixel(&mut fb, black_box(400), black_box(300), 0xFFFFFFFF);
        });
    });
}

fn bench_plot_many_pixels(c: &mut Criterion) {
    c.bench_function("plot_1000_pixels", |b| {
        let mut fb = Framebuffer::new(800, 600);
        b.iter(|| {
            for i in 0..1000 {
                plot_pixel(&mut fb, i % 800, i / 800, 0xFFFFFFFF);
            }
        });
    });
}

criterion_group!(benches,
    bench_clear,
    bench_plot_pixel,
    bench_plot_many_pixels
);
criterion_main!(benches);
