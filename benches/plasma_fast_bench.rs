use abrash_render::procedural::{plasma, plasma_fast};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_plasma_fast(c: &mut Criterion) {
    let mut group = c.benchmark_group("plasma_fast_64x64");

    group.bench_function("plasma_original", |b| {
        b.iter(|| {
            black_box(plasma(64, 64).unwrap());
        });
    });

    group.bench_function("plasma_fast", |b| {
        b.iter(|| {
            black_box(plasma_fast(64, 64).unwrap());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_plasma_fast);
criterion_main!(benches);
