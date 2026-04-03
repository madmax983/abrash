use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_render::experimental::plasma::apply_plasma;
use abrash_core::framebuffer::Framebuffer;

fn bench_plasma(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let time = 1.0;
    let scale = 0.05;

    c.bench_function("plasma_800x600", |b| {
        b.iter(|| {
            apply_plasma(black_box(&mut fb), black_box(time), black_box(scale));
        });
    });
}

criterion_group!(benches, bench_plasma);
criterion_main!(benches);
