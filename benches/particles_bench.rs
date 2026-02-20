use abrash::framebuffer::Framebuffer;
use abrash::particles::{ParticleSystem, Particle};
use abrash::math::{Vec3, Mat4};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_particles_render(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();
    let texture = Texture::checkered(64, 64, 0xFFFFFFFF, 0xFF000000).unwrap();
    let mut sys = ParticleSystem::new(10_000, texture);

    // Pre-populate particles
    for i in 0..5000 {
        sys.particles.push(Particle::new(
            Vec3::new((i % 100) as f32 - 50.0, (i / 100) as f32 - 25.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            1.0,
            0.5,
            0xFFFFFFFF,
        ));
    }

    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.5, 1.33, 0.1, 100.0);

    c.bench_function("particles_render_5000", |b| {
        b.iter(|| {
            zb.clear();
            fb.clear(0);
            sys.render(&mut fb, &mut zb, black_box(view), black_box(proj));
        });
    });
}

criterion_group!(benches, bench_particles_render);
criterion_main!(benches);
