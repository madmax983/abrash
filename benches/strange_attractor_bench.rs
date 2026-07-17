use abrash_render::experimental::strange_attractor::{LorenzAttractor, Particle};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_strange_attractor(c: &mut Criterion) {
    let mut group = c.benchmark_group("strange_attractor");

    group.bench_function("run_1000_steps_10k_particles", |b| {
        let mut particles = Vec::with_capacity(10_000);
        for i in 0..10_000 {
            particles.push(Particle {
                x: (i as f32) / 1000.0,
                y: (i as f32) / 1000.0,
                z: (i as f32) / 1000.0,
            });
        }

        let mut attractor = LorenzAttractor::new(10.0, 28.0, 8.0 / 3.0, 0.01, particles);

        b.iter(|| {
            attractor.run_steps(1000);
            black_box(&attractor.particles[0].x);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_strange_attractor);
criterion_main!(benches);
