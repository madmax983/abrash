use abrash_render::experimental::attractor::{AttractorSystem, AttractorType, Particle};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_attractor(c: &mut Criterion) {
    let mut system = AttractorSystem::new(AttractorType::default());
    for i in 0..10_000 {
        system.particles.push(Particle::new([
            i as f32 * 0.1,
            i as f32 * 0.2,
            i as f32 * 0.3,
        ]));
    }

    c.bench_function("attractor_serial_update", |b| {
        b.iter(|| {
            system.update(black_box(0.01));
        })
    });

    c.bench_function("attractor_batched_update", |b| {
        b.iter(|| {
            system.run_steps(black_box(0.01), black_box(10));
        })
    });
}

criterion_group!(benches, bench_attractor);
criterion_main!(benches);
