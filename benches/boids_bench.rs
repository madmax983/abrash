use abrash::experimental::boids::{Boid, Flock, FlockConfig};
use abrash::math::Vec3;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_boids_update(c: &mut Criterion) {
    let mut flock = Flock::new(FlockConfig::default());

    // Add 500 boids for the benchmark
    for i in 0..500 {
        flock.add_boid(Boid::new(
            Vec3::new(
                (i as f32 * 13.0) % 50.0,
                (i as f32 * 17.0) % 50.0,
                (i as f32 * 19.0) % 50.0,
            ),
            Vec3::new(
                (i as f32 % 5.0) - 2.5,
                (i as f32 % 3.0) - 1.5,
                (i as f32 % 7.0) - 3.5,
            ),
        ));
    }

    c.bench_function("boids_update", |b| {
        b.iter(|| {
            flock.update(black_box(0.016));
        });
    });
}

criterion_group!(benches, bench_boids_update);
criterion_main!(benches);
