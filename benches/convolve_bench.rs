use criterion::{Criterion, black_box, criterion_group, criterion_main};

// The original implementation for comparison
pub fn convolve_1d_original(signal: &[f32], kernel: &[f32]) -> Vec<f32> {
    if signal.is_empty() || kernel.is_empty() {
        return Vec::new();
    }
    let out_len = signal.len() + kernel.len() - 1;
    let mut out = vec![0.0_f32; out_len];
    for (i, &s) in signal.iter().enumerate() {
        for (j, &k) in kernel.iter().enumerate() {
            out[i + j] += s * k;
        }
    }
    out
}

use abrash_core::math::funcs::convolve_1d;

fn bench_convolve(c: &mut Criterion) {
    let mut group = c.benchmark_group("Convolve 1D");

    // Create some dummy data
    let signal = vec![1.0_f32; 1000];
    let kernel = vec![0.5_f32; 100];

    group.bench_function("original", |b| {
        b.iter(|| convolve_1d_original(black_box(&signal), black_box(&kernel)))
    });

    group.bench_function("optimized", |b| {
        b.iter(|| convolve_1d(black_box(&signal), black_box(&kernel)))
    });

    group.finish();
}

criterion_group!(benches, bench_convolve);
criterion_main!(benches);
