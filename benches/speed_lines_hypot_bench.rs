use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_hypot_vs_sqrt(c: &mut Criterion) {
    let dx = 300.0f32;
    let dy = 400.0f32;

    let mut group = c.benchmark_group("speed_lines_distance_calc");

    group.bench_function("hypot", |b| {
        b.iter(|| {
            let dist = black_box(dx).hypot(black_box(dy));
            black_box(dist);
        });
    });

    group.bench_function("sqrt", |b| {
        b.iter(|| {
            let x = black_box(dx);
            let y = black_box(dy);
            #[allow(clippy::imprecise_flops)]
            let dist = (x * x + y * y).sqrt();
            black_box(dist);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_hypot_vs_sqrt);
criterion_main!(benches);
