use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::particles::ParticleSystem;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_particles_render(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();
    let texture = Texture::checkered(32, 32, 0xFFFFFFFF, 0xFF000000).unwrap();

    // Create a particle system with 1000 particles
    let mut particles = ParticleSystem::new(1000, texture);
    particles.emission_rate = 1000.0;

    // Simulate some updates to spawn particles
    for _ in 0..100 {
        particles.update(0.016);
    }

    // Camera setup
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0);

    c.bench_function("particles_render_1000", |b| {
        b.iter(|| {
            fb.clear(0);
            zb.clear();
            particles.render(&mut fb, &mut zb, view, proj);
        });
    });
}

criterion_group!(benches, bench_particles_render);
criterion_main!(benches);
