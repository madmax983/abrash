use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::oil_paint::apply_oil_paint;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn oil_paint_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    for i in 0..(800 * 600) {
        fb.as_mut_slice()[i] = (i as u32) | 0xFF00_0000;
    }
    c.bench_function("oil_paint 800x600 radius 3", |b| {
        b.iter(|| {
            apply_oil_paint(black_box(&mut fb), black_box(3), black_box(20));
        });
    });
}
criterion_group!(benches, oil_paint_benchmark);
criterion_main!(benches);
