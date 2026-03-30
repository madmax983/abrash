use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[inline(always)]
fn fast_atan2(y: f32, x: f32) -> f32 {
    let abs_y = y.abs() + 1e-10;
    let abs_x = x.abs() + 1e-10;
    let r = (abs_x - abs_y) / (abs_x + abs_y);
    let mut angle = std::f32::consts::FRAC_PI_4 - std::f32::consts::FRAC_PI_4 * r;
    if x < 0.0 {
        angle = std::f32::consts::PI - angle;
    }
    if y < 0.0 {
        angle = -angle;
    }
    angle
}

fn bench_atan2(c: &mut Criterion) {
    let inputs: Vec<(f32, f32)> = (0..1000).map(|i| {
        let x = (i as f32) - 500.0;
        let y = (i as f32) * 0.5 - 250.0;
        (x, y)
    }).collect();

    let mut group = c.benchmark_group("atan2");

    group.bench_function("std_atan2", |b| {
        b.iter(|| {
            for &(x, y) in &inputs {
                black_box(y.atan2(x));
            }
        });
    });

    group.bench_function("fast_atan2", |b| {
        b.iter(|| {
            for &(x, y) in &inputs {
                black_box(fast_atan2(y, x));
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_atan2);
criterion_main!(benches);
