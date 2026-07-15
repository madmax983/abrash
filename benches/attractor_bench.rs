use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::math::Vec3;
use abrash_render::experimental::attractor::LorenzAttractor;

fn bench_lorenz_step(c: &mut Criterion) {
    let mut attractor = LorenzAttractor::new(Vec3::new(1.0, 1.0, 1.0));
    c.bench_function("lorenz_step_100", |b| {
        b.iter(|| {
            for _ in 0..100 {
                attractor.step(black_box(0.01));
            }
        });
    });

    c.bench_function("lorenz_run_steps_100", |b| {
        b.iter(|| {
            attractor.run_steps(black_box(0.01), black_box(100));
        });
    });
}

criterion_group!(benches, bench_lorenz_step);
criterion_main!(benches);
