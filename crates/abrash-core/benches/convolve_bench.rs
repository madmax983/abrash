use abrash_core::math::funcs::convolve_1d;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_convolve(c: &mut Criterion) {
    let signal: Vec<f32> = (0..1024).map(|i| i as f32).collect();
    let kernel: Vec<f32> = (0..64).map(|i| i as f32).collect();
    c.bench_function("convolve_1d", |b| {
        b.iter(|| convolve_1d(black_box(&signal), black_box(&kernel)))
    });
}

criterion_group!(benches, bench_convolve);
criterion_main!(benches);
