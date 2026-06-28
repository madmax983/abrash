use abrash_render::experimental::boids::{Boid, Flock, FlockConfig};
use abrash_core::math::Vec3;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rand::Rng;

fn boids_benchmark(c: &mut Criterion) {
    let mut flock = Flock::new(FlockConfig::default());
    let mut rng = rand::thread_rng();

    for _ in 0..1000 {
        flock.add_boid(Boid::new(
            Vec3::new(
                rng.gen_range(0.0..100.0),
                rng.gen_range(0.0..100.0),
                rng.gen_range(0.0..100.0),
            ),
            Vec3::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            ),
        ));
    }

    c.bench_function("boids update 1000", |b| {
        b.iter(|| {
            flock.update(black_box(1.0 / 60.0));
        });
    });
}

criterion_group!(benches, boids_benchmark);
criterion_main!(benches);
