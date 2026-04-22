use criterion::{Criterion, black_box, criterion_group, criterion_main};
use abrash_core::math::{idct_ii, dct_ii};

fn bench_sin(c: &mut Criterion) {
    let data: Vec<f32> = (0..1000).map(|i| i as f32 * 0.1).collect();

    // Use an immutable array so the benchmark doesn't optimize it to `0` or out-of-bounds math over iterations
    c.bench_function("std_sin_loop", |b| {
        b.iter(|| {
            for &val in &data {
                black_box(val.sin());
            }
        });
    });

    c.bench_function("std_sin_cos_loop", |b| {
        b.iter(|| {
            for &val in &data {
                black_box(val.sin() + val.cos());
            }
        });
    });

    c.bench_function("std_sin_cos_pair_loop", |b| {
        b.iter(|| {
            for &val in &data {
                let (s, c) = val.sin_cos();
                black_box(s + c);
            }
        });
    });
}



fn bench_dct(c: &mut Criterion) {
    let mut group = c.benchmark_group("dct");

    let dct_signal: Vec<f32> = (0..256).map(|i| i as f32).collect();
    group.bench_function("dct_ii", |b| {
        b.iter(|| dct_ii(black_box(&dct_signal)))
    });

    let dct_coeffs: Vec<f32> = (0..256).map(|i| i as f32).collect();
    group.bench_function("idct_ii", |b| {
        b.iter(|| idct_ii(black_box(&dct_coeffs)))
    });

    group.finish();
}

criterion_group!(benches, bench_sin, bench_dct);
criterion_main!(benches);
