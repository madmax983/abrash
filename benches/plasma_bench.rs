use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::plasma::apply_plasma;

fn bench_plasma(c: &mut Criterion) {
    let mut group = c.benchmark_group("Plasma Benchmark");

    // Benchmark at common resolution
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    group.bench_function(format!("plasma_{}x{}", width, height), |b| {
        b.iter(|| apply_plasma(black_box(&mut fb), black_box(0.0), black_box(0.05)))
    });

    group.finish();
}

criterion_group!(benches, bench_plasma);
criterion_main!(benches);
