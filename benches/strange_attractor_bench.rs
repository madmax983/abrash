use abrash_core::math::Vec3;
use abrash_render::experimental::strange_attractor::{AttractorType, Particle, StrangeAttractor};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn strange_attractor_benchmark(c: &mut Criterion) {
    let mut particles = Vec::with_capacity(10_000);
    for i in 0..10_000 {
        particles.push(Particle {
            position: Vec3::new(i as f32 * 0.01, 1.0, 1.0),
            color: 0,
        });
    }

    let mut sim = StrangeAttractor::new(
        particles,
        AttractorType::Lorenz {
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
        },
        0.01,
    );

    c.bench_function("strange_attractor_step_100_batched", |b| {
        b.iter(|| sim.run_steps(black_box(100)));
    });
}

criterion_group!(benches, strange_attractor_benchmark);
criterion_main!(benches);
