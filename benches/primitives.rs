use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash::framebuffer::Framebuffer;
use abrash::primitives::{draw_hline, draw_line, draw_vline, fill_triangle, plot_pixel};
use abrash::shapes::Triangle;
use abrash::math::Vec2;

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

fn bench_draw_line_horizontal(c: &mut Criterion) {
    c.bench_function("draw_line_horizontal_100px", |b| {
        let mut fb = Framebuffer::new(800, 600);
        b.iter(|| {
            draw_line(&mut fb, black_box(100), black_box(300), black_box(200), black_box(300), 0xFFFFFFFF);
        });
    });
}

fn bench_draw_line_diagonal(c: &mut Criterion) {
    c.bench_function("draw_line_diagonal_100px", |b| {
        let mut fb = Framebuffer::new(800, 600);
        b.iter(|| {
            draw_line(&mut fb, black_box(100), black_box(100), black_box(200), black_box(200), 0xFFFFFFFF);
        });
    });
}

fn bench_draw_line_long(c: &mut Criterion) {
    c.bench_function("draw_line_diagonal_500px", |b| {
        let mut fb = Framebuffer::new(800, 600);
        b.iter(|| {
            draw_line(&mut fb, black_box(50), black_box(50), black_box(550), black_box(550), 0xFFFFFFFF);
        });
    });
}

fn bench_draw_hline(c: &mut Criterion) {
    c.bench_function("draw_hline_100px", |b| {
        let mut fb = Framebuffer::new(800, 600);
        b.iter(|| {
            draw_hline(&mut fb, black_box(100), black_box(200), black_box(300), 0xFFFFFFFF);
        });
    });
}

fn bench_draw_vline(c: &mut Criterion) {
    c.bench_function("draw_vline_100px", |b| {
        let mut fb = Framebuffer::new(800, 600);
        b.iter(|| {
            draw_vline(&mut fb, black_box(400), black_box(100), black_box(200), 0xFFFFFFFF);
        });
    });
}

fn bench_line_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("line_comparison");

    group.bench_function("hline_optimized_100px", |b| {
        let mut fb = Framebuffer::new(800, 600);
        b.iter(|| draw_hline(&mut fb, black_box(100), black_box(200), black_box(300), 0xFFFFFFFF));
    });

    group.bench_function("vline_optimized_100px", |b| {
        let mut fb = Framebuffer::new(800, 600);
        b.iter(|| draw_vline(&mut fb, black_box(400), black_box(100), black_box(200), 0xFFFFFFFF));
    });

    group.finish();
}

fn bench_fill_triangle_small(c: &mut Criterion) {
    c.bench_function("fill_triangle_50px", |b| {
        let mut fb = Framebuffer::new(800, 600);
        let tri = Triangle::new(
            Vec2::new(400.0, 275.0),
            Vec2::new(425.0, 325.0),
            Vec2::new(375.0, 325.0),
        );
        b.iter(|| fill_triangle(&mut fb, black_box(&tri), 0xFFFFFFFF));
    });
}

fn bench_fill_triangle_large(c: &mut Criterion) {
    c.bench_function("fill_triangle_200px", |b| {
        let mut fb = Framebuffer::new(800, 600);
        let tri = Triangle::new(
            Vec2::new(400.0, 200.0),
            Vec2::new(500.0, 400.0),
            Vec2::new(300.0, 400.0),
        );
        b.iter(|| fill_triangle(&mut fb, black_box(&tri), 0xFFFFFFFF));
    });
}

criterion_group!(benches,
    bench_clear,
    bench_plot_pixel,
    bench_plot_many_pixels,
    bench_draw_line_horizontal,
    bench_draw_line_diagonal,
    bench_draw_line_long,
    bench_draw_hline,
    bench_draw_vline,
    bench_line_comparison,
    bench_fill_triangle_small,
    bench_fill_triangle_large
);
criterion_main!(benches);
