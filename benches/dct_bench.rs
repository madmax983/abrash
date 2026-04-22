use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::math::{dct_ii, idct_ii};

fn bench_dct(c: &mut Criterion) {
    let mut group = c.benchmark_group("dct");
    group.sample_size(100);

    // Common signal size for image blocks is 8 (8x8 blocks, so 8 elements per 1D pass)
    let signal_8 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

    // Test a larger signal as well
    let signal_64: Vec<f32> = (0..64).map(|x| x as f32).collect();

    group.bench_function("dct_ii_8", |b| {
        b.iter(|| dct_ii(black_box(&signal_8)));
    });

    group.bench_function("dct_ii_64", |b| {
        b.iter(|| dct_ii(black_box(&signal_64)));
    });

    let coeffs_8 = dct_ii(&signal_8);
    let coeffs_64 = dct_ii(&signal_64);

    group.bench_function("idct_ii_8", |b| {
        b.iter(|| idct_ii(black_box(&coeffs_8)));
    });

    group.bench_function("idct_ii_64", |b| {
        b.iter(|| idct_ii(black_box(&coeffs_64)));
    });

    group.finish();
}

criterion_group!(benches, bench_dct);
criterion_main!(benches);
