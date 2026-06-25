use abrash::math::Vec3;
use abrash::particles::{Particle, ParticleSystem};
use abrash::texture::Texture;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_particles_physics(c: &mut Criterion) {
    c.bench_function("particles_physics_50k", |b| {
        let texture = Texture::new(32, 32).unwrap();

        b.iter_batched(
            || {
                let mut sys = ParticleSystem::new(50000, texture.clone());
                sys.emission_rate = 0.0;
                // Add 50k particles
                for i in 0..50000 {
                    sys.particles.push(Particle::new(
                        Vec3::ZERO,
                        Vec3::ZERO,
                        if i % 10 == 0 { 0.5 } else { 2.0 }, // 10% die early
                        0.5,
                        0xFFFF_FFFF,
                    ));
                }
                sys
            },
            |mut sys| {
                sys.update(1.0);
                black_box(sys.particles.len());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_particles_update(c: &mut Criterion) {
    c.bench_function("particles_update_emit", |b| {
        let texture = Texture::new(32, 32).unwrap();
        b.iter(|| {
            let mut sys = ParticleSystem::new(0, texture.clone());
            sys.emission_rate = 50000.0;
            sys.update(1.0); // Emits 50000 particles
            black_box(sys);
        });
    });
}

criterion_group!(benches, bench_particles_physics, bench_particles_update);
criterion_main!(benches);
