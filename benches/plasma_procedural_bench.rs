use abrash_render::procedural::plasma;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_plasma_procedural(c: &mut Criterion) {
    c.bench_function("plasma_procedural_800x600", |b| {
        b.iter(|| {
            let _ = plasma(black_box(800), black_box(600)).unwrap();
        });
    });
}

criterion_group!(benches, bench_plasma_procedural);
criterion_main!(benches);
