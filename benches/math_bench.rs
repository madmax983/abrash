use criterion::{Criterion, black_box, criterion_group, criterion_main};

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

criterion_group!(benches, bench_sin);
criterion_main!(benches);
