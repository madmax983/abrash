use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::{draw_ellipse, fill_ellipse};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_draw_ellipse(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let color = 0xFFFF_FFFF;

    c.bench_function("draw_ellipse_100x50", |b| {
        b.iter(|| {
            draw_ellipse(
                &mut fb,
                black_box(400),
                black_box(300),
                black_box(100),
                black_box(50),
                black_box(color),
            );
        });
    });

    c.bench_function("draw_ellipse_oob", |b| {
        b.iter(|| {
            draw_ellipse(
                &mut fb,
                black_box(-50),
                black_box(-50),
                black_box(100),
                black_box(50),
                black_box(color),
            );
        });
    });
}

fn bench_fill_ellipse(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let color = 0xFFFF_FFFF;

    c.bench_function("fill_ellipse_100x50", |b| {
        b.iter(|| {
            fill_ellipse(
                &mut fb,
                black_box(400),
                black_box(300),
                black_box(100),
                black_box(50),
                black_box(color),
            );
        });
    });

    c.bench_function("fill_ellipse_oob", |b| {
        b.iter(|| {
            fill_ellipse(
                &mut fb,
                black_box(-50),
                black_box(-50),
                black_box(100),
                black_box(50),
                black_box(color),
            );
        });
    });
}

criterion_group!(benches, bench_draw_ellipse, bench_fill_ellipse);
criterion_main!(benches);
