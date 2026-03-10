use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash::experimental::boids::{Boid, BoidsSimulation};
use abrash::math::Vec3;
use rand::Rng;

fn create_random_boids(count: usize) -> Vec<Boid> {
    let mut rng = rand::thread_rng();
    let mut boids = Vec::with_capacity(count);
    for _ in 0..count {
        boids.push(Boid::new(
            Vec3::new(
                rng.gen_range(-40.0..40.0),
                rng.gen_range(-40.0..40.0),
                rng.gen_range(-40.0..40.0),
            ),
            Vec3::new(
                rng.gen_range(-5.0..5.0),
                rng.gen_range(-5.0..5.0),
                rng.gen_range(-5.0..5.0),
            ),
        ));
    }
    boids
}

fn bench_boids_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("boids_simulation");

    // Benchmark a realistic flock size (e.g., 500 boids)
    let flock_size = 500;
    let initial_boids = create_random_boids(flock_size);
    let mut sim = BoidsSimulation::new(initial_boids.clone());

    group.bench_function(format!("step_{}", flock_size), |b| {
        b.iter(|| {
            sim.step(black_box(0.016)); // ~60 FPS step
        })
    });

    group.finish();
}

criterion_group!(benches, bench_boids_step);
criterion_main!(benches);
