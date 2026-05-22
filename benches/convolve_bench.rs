use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::math::funcs::convolve_1d;

fn bench_convolve(c: &mut Criterion) {
    let mut group = c.benchmark_group("convolve_1d");
    let signal: Vec<f32> = (0..1000).map(|x| x as f32).collect();
    let kernel: Vec<f32> = (0..50).map(|x| x as f32).collect();

    group.bench_function("convolve_1d", |b| {
        b.iter(|| convolve_1d(black_box(&signal), black_box(&kernel)));
    });
    group.finish();
}

criterion_group!(benches, bench_convolve);
criterion_main!(benches);
