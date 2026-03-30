use criterion::{criterion_group, criterion_main, Criterion};
use abrash_render::procedural::plasma;

fn bench_plasma(c: &mut Criterion) {
    c.bench_function("procedural_plasma_800x600", |b| {
        b.iter(|| {
            plasma(800, 600).unwrap()
        })
    });
}

criterion_group!(benches, bench_plasma);
criterion_main!(benches);
