use criterion::{Criterion, black_box, criterion_group, criterion_main};

use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::{draw_circle, fill_circle};

fn bench_draw_circle(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();

    c.bench_function("draw_circle_r10", |b| {
        b.iter(|| {
            draw_circle(
                &mut fb,
                black_box(400),
                black_box(300),
                black_box(10),
                black_box(0xFFFF_FFFF),
            );
        });
    });

    c.bench_function("draw_circle_r100", |b| {
        b.iter(|| {
            draw_circle(
                &mut fb,
                black_box(400),
                black_box(300),
                black_box(100),
                black_box(0xFFFF_FFFF),
            );
        });
    });

    c.bench_function("draw_circle_r100_out_of_bounds", |b| {
        b.iter(|| {
            draw_circle(
                &mut fb,
                black_box(-50),
                black_box(300),
                black_box(100),
                black_box(0xFFFF_FFFF),
            );
        });
    });
}

fn bench_fill_circle(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();

    c.bench_function("fill_circle_r10", |b| {
        b.iter(|| {
            fill_circle(
                &mut fb,
                black_box(400),
                black_box(300),
                black_box(10),
                black_box(0xFFFF_FFFF),
            );
        });
    });

    c.bench_function("fill_circle_r100", |b| {
        b.iter(|| {
            fill_circle(
                &mut fb,
                black_box(400),
                black_box(300),
                black_box(100),
                black_box(0xFFFF_FFFF),
            );
        });
    });

    c.bench_function("fill_circle_r100_out_of_bounds", |b| {
        b.iter(|| {
            fill_circle(
                &mut fb,
                black_box(-50),
                black_box(300),
                black_box(100),
                black_box(0xFFFF_FFFF),
            );
        });
    });
}

criterion_group!(benches, bench_draw_circle, bench_fill_circle);
criterion_main!(benches);
