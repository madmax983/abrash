use abrash_core::texture::Texture;
use abrash_render::particles::ParticleSystem;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_particles_update(c: &mut Criterion) {
    let tex = Texture::new(8, 8).unwrap();
    let mut sys = ParticleSystem::new(100_000, tex);
    sys.emission_rate = 100_000.0;

    // warm up / fill
    sys.update(1.0);

    c.bench_function("particles_update_100k", |b| {
        b.iter(|| {
            sys.update(black_box(0.016));
        });
    });
}

criterion_group!(benches, bench_particles_update);
criterion_main!(benches);
