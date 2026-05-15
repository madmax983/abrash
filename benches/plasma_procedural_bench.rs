use abrash_render::procedural::plasma;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_plasma_procedural(c: &mut Criterion) {
    c.bench_function("plasma_procedural_64x64", |b| {
        b.iter(|| {
            black_box(plasma(64, 64).unwrap());
        });
    });
}

criterion_group!(benches, bench_plasma_procedural);
criterion_main!(benches);
