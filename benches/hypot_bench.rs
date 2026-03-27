use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

fn bench_hypot_f32(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(42);
    let coords: Vec<(f32, f32)> = (0..10_000)
        .map(|_| {
            (
                rng.gen_range(-1000.0..1000.0),
                rng.gen_range(-1000.0..1000.0),
            )
        })
        .collect();

    c.bench_function("math/f32_hypot", |b| {
        b.iter(|| {
            for (dx, dy) in &coords {
                let _ = black_box(dx.hypot(*dy));
            }
        });
    });
}

fn bench_manual_sqrt(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(42);
    let coords: Vec<(f32, f32)> = (0..10_000)
        .map(|_| {
            (
                rng.gen_range(-1000.0..1000.0),
                rng.gen_range(-1000.0..1000.0),
            )
        })
        .collect();

    c.bench_function("math/manual_sqrt", |b| {
        b.iter(|| {
            for (dx, dy) in &coords {
                #[allow(clippy::imprecise_flops)]
                let _ = black_box((dx * dx + dy * dy).sqrt());
            }
        });
    });
}

criterion_group!(benches, bench_hypot_f32, bench_manual_sqrt);
criterion_main!(benches);
