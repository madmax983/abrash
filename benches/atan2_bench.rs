#![cfg(feature = "nova")]

use abrash::experimental::kaleidoscope::fast_atan2;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_atan2(c: &mut Criterion) {
    let mut group = c.benchmark_group("atan2_comparison");

    group.bench_function("std_atan2", |b| {
        b.iter(|| {
            black_box(black_box(1.5f32).atan2(black_box(0.5f32)));
        });
    });

    group.bench_function("fast_atan2", |b| {
        b.iter(|| {
            black_box(fast_atan2(black_box(1.5f32), black_box(0.5f32)));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_atan2);
criterion_main!(benches);
