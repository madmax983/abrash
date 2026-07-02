use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[inline(always)]
fn fast_powi_32(mut x: f32) -> f32 {
    x *= x;
    x *= x;
    x *= x;
    x *= x;
    x *= x;
    x
}

fn powi_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("powi_32");

    // In our raytracer, we call it on the dot product of reflect_dir and view_dir
    group.bench_function("std_powi", |b| {
        let x = 0.5f32;
        b.iter(|| {
            black_box(black_box(x).powi(32));
        })
    });

    group.bench_function("fast_powi", |b| {
        let x = 0.5f32;
        b.iter(|| {
            black_box(fast_powi_32(black_box(x)));
        })
    });

    group.finish();
}

criterion_group!(benches, powi_benchmark);
criterion_main!(benches);
