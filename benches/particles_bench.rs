use criterion::{criterion_group, criterion_main, Criterion};
use abrash_core::math::Vec3;
use abrash_render::particles::ParticleSystem;
use abrash_core::texture::Texture;

fn bench_particles(c: &mut Criterion) {
    let texture = Texture::new(2, 2).unwrap();
    let mut sys = ParticleSystem::new(10000, texture);
    sys.emission_rate = 10000.0;

    // warm up
    for _ in 0..100 {
        sys.update(0.016);
    }

    c.bench_function("particles_update_10000", |b| {
        b.iter(|| {
            sys.update(0.016);
        })
    });
}

criterion_group!(benches, bench_particles);
criterion_main!(benches);
