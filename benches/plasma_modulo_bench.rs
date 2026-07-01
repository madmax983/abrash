use criterion::{criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::plasma::apply_plasma;

fn plasma_modulo_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();

    c.bench_function("plasma_modulo_optimized_800x600", |b| {
        b.iter(|| {
            apply_plasma(&mut fb, 1.0, 0.05);
            std::hint::black_box(&fb);
        });
    });
}

criterion_group!(benches, plasma_modulo_benchmark);
criterion_main!(benches);
