use criterion::{criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::chladni::generate_chladni_pattern;

fn chladni_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    c.bench_function("generate_chladni_pattern", |b| {
        b.iter(|| generate_chladni_pattern(&mut fb, 2.0, 3.0, 0.0));
    });
}

criterion_group!(benches, chladni_benchmark);
criterion_main!(benches);
