use criterion::{criterion_group, criterion_main, Criterion};
use abrash::particles::ParticleSystem;
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::math::Mat4;
use abrash::texture::Texture;

fn particles_render_benchmark(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let texture = Texture::new(32, 32).unwrap();
    let mut sys = ParticleSystem::new(1000, texture);

    // Populate particles
    sys.emission_rate = 10000.0; // Emit lots of particles
    sys.update(0.1); // Should emit ~1000 particles (max capacity)

    let view = Mat4::look_at(
        abrash::math::Vec3::new(0.0, 0.0, 10.0),
        abrash::math::Vec3::new(0.0, 0.0, 0.0),
        abrash::math::Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(std::f32::consts::PI / 4.0, width as f32 / height as f32, 0.1, 100.0);

    c.bench_function("particles_render_1000", |b| {
        b.iter(|| {
            fb.clear(0);
            zb.clear();
            sys.render(&mut fb, &mut zb, view, proj);
        })
    });
}

criterion_group!(benches, particles_render_benchmark);
criterion_main!(benches);
