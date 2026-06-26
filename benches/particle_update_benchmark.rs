use abrash_core::texture::Texture;
use abrash_render::particles::ParticleSystem;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_particle_update(c: &mut Criterion) {
    c.bench_function("particles_update_50k", |b| {
        let texture = Texture::new(1, 1).unwrap();
        let mut sys = ParticleSystem::new(50000, texture);
        sys.emission_rate = 50000.0;
        sys.start_life = 100.0;
        sys.spread = 10.0;
        sys.update(1.0);
        assert!(sys.particles.len() >= 40000);

        b.iter(|| {
            sys.update(black_box(0.016));
        });
    });
}

criterion_group!(benches, bench_particle_update);
criterion_main!(benches);
