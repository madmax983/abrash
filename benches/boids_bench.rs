use abrash_core::math::Vec3;
use abrash_render::experimental::boids::{Boid, Flock, FlockConfig};
use criterion::{Criterion, criterion_group, criterion_main};

pub fn boids_benchmark(c: &mut Criterion) {
    let mut flock = Flock::new(FlockConfig::default());
    for i in 0..1000 {
        flock.add_boid(Boid::new(
            Vec3::new(i as f32, i as f32, i as f32),
            Vec3::new(1.0, 1.0, 1.0),
        ));
    }

    c.bench_function("boids_update", |b| {
        b.iter(|| {
            flock.update(0.016);
        });
    });
}

criterion_group!(benches, boids_benchmark);
criterion_main!(benches);
