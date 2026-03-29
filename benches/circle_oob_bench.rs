use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::{draw_circle, fill_circle};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_draw_circle_oob(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();

    c.bench_function("draw_circle_negative_radius", |b| {
        b.iter(|| {
            // This should safely return without panic/OOB
            draw_circle(
                &mut fb,
                black_box(0),
                black_box(0),
                black_box(-100),
                black_box(0xFFFF_FFFF),
            );
        });
    });

    c.bench_function("fill_circle_negative_radius", |b| {
        b.iter(|| {
            // This should safely return without panic/OOB
            fill_circle(
                &mut fb,
                black_box(0),
                black_box(0),
                black_box(-100),
                black_box(0xFFFF_FFFF),
            );
        });
    });
}
criterion_group!(benches, bench_draw_circle_oob);
criterion_main!(benches);
