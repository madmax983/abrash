use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "nova")]
use abrash::experimental::kaleidoscope::fast_atan2;

#[cfg(feature = "nova")]
fn bench_fast_atan2(c: &mut Criterion) {
    let mut group = c.benchmark_group("atan2");

    // Generate some input data
    let mut inputs = Vec::with_capacity(1000);
    for y in -15..15 {
        for x in -15..15 {
            inputs.push((y as f32 * 1.5, x as f32 * 1.5));
        }
    }

    group.bench_function("std_atan2", |b| {
        b.iter(|| {
            for (y, x) in &inputs {
                black_box(black_box(*y).atan2(black_box(*x)));
            }
        });
    });

    group.bench_function("fast_atan2", |b| {
        b.iter(|| {
            for (y, x) in &inputs {
                black_box(fast_atan2(black_box(*y), black_box(*x)));
            }
        });
    });

    group.finish();
}

#[cfg(not(feature = "nova"))]
fn bench_fast_atan2(c: &mut Criterion) {
    let mut group = c.benchmark_group("atan2");
    group.bench_function("stub", |b| b.iter(|| black_box(0)));
    group.finish();
}

criterion_group!(benches, bench_fast_atan2);
criterion_main!(benches);
