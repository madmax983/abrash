use criterion::{Criterion, black_box, criterion_group, criterion_main};
use abrash_core::math::{henyey_greenstein, Vec2};
use abrash_core::sdf::parabola_2d;

fn bench_henyey_greenstein(c: &mut Criterion) {
    let data: Vec<f32> = (0..1000).map(|i| i as f32 * 0.001 * std::f32::consts::PI - std::f32::consts::PI / 2.0).collect();

    c.bench_function("henyey_greenstein", |b| {
        b.iter(|| {
            for &val in &data {
                black_box(henyey_greenstein(val, 0.5));
            }
        });
    });
}

fn bench_parabola_2d(c: &mut Criterion) {
    let data: Vec<Vec2> = (0..1000).map(|i| Vec2::new(i as f32 * 0.1, i as f32 * 0.1)).collect();

    c.bench_function("parabola_2d", |b| {
        b.iter(|| {
            for &val in &data {
                black_box(parabola_2d(val, 0.5));
            }
        });
    });
}

criterion_group!(benches, bench_henyey_greenstein, bench_parabola_2d);
criterion_main!(benches);
