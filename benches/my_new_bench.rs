use criterion::{criterion_group, criterion_main, Criterion};

fn bench_placeholder(c: &mut Criterion) {
    let mut group = c.benchmark_group("placeholder");
    group.bench_function("placeholder", |b| b.iter(|| 1 + 1));
    group.finish();
}

criterion_group!(benches, bench_placeholder);
criterion_main!(benches);
