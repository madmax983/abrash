use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::rect::fill_rounded_rect;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_fill_rounded_rect_offscreen(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();

    c.bench_function("fill_rounded_rect_massive_offscreen", |b| {
        b.iter(|| {
            fill_rounded_rect(
                black_box(&mut fb),
                black_box(-2_147_483_640),
                black_box(-2_147_483_640),
                black_box(2_147_483_000),
                black_box(2_147_483_000),
                black_box(1_073_741_000),
                black_box(0xFFFF_FFFF),
            );

        });

    });
}

criterion_group!(benches, bench_fill_rounded_rect_offscreen);
criterion_main!(benches);
