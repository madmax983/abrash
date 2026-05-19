use abrash_render::procedural::plasma;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_plasma_procedural(c: &mut Criterion) {
    let mut group = c.benchmark_group("plasma_procedural");
    group.bench_function("64x64", |b| {
        b.iter(|| {
            black_box(plasma(64, 64).unwrap());
        });
    });
    group.bench_function("128x128", |b| {
        b.iter(|| {
            black_box(plasma(128, 128).unwrap());
        });
    });
    group.finish();
}

criterion_group!(benches, bench_plasma_procedural);
criterion_main!(benches);
