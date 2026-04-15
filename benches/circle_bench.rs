#![allow(unused)]
use criterion::{Criterion, black_box, criterion_group, criterion_main};

use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::fill_circle;

fn bench_fill_circle(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();

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
}
criterion_group!(benches, bench_fill_circle);
criterion_main!(benches);
