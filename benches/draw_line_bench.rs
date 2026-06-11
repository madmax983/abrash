#![allow(unused)]
use criterion::{Criterion, black_box, criterion_group, criterion_main};

use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::draw_line;

fn bench_draw_line(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();

    c.bench_function("draw_line_horizontal", |b| {
        b.iter(|| {
            draw_line(
                &mut fb,
                black_box(10),
                black_box(300),
                black_box(790),
                black_box(300),
                black_box(0xFFFF_FFFF),
            );
        });
    });

    c.bench_function("draw_line_diagonal", |b| {
        b.iter(|| {
            draw_line(
                &mut fb,
                black_box(10),
                black_box(10),
                black_box(790),
                black_box(590),
                black_box(0xFFFF_FFFF),
            );
        });
    });
}

criterion_group!(benches, bench_draw_line);
criterion_main!(benches);
