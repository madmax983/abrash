use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::particles::{Particle, ParticleSystem};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_colored_particles_render(c: &mut Criterion) {
    c.bench_function("colored_particles_render_50k", |b| {
        let width = 800;
        let height = 600;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Create a texture for particles
        let texture = Texture::new(32, 32).unwrap();

        // Initialize particle system
        let mut sys = ParticleSystem::new(50000, texture);

        // Manually push 50k particles with random colors
        for i in 0..50000 {
            let x = (i % 100) as f32 - 50.0;
            let y = (i / 100 % 100) as f32 - 50.0;
            let z = (i / 10000) as f32;

            // Randomish color
            let r = (i * 123) % 255;
            let g = (i * 456) % 255;
            let b = (i * 789) % 255;
            let color: u32 = 0xFF000000 | (r << 16) | (g << 8) | b;

            sys.particles.push(Particle::new(
                Vec3::new(x, y, z),
                Vec3::new(0.0, 0.0, 0.0),
                10.0,
                0.5,
                color as u32,
            ));
        }

        // Setup camera
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 100.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 1000.0);

        b.iter(|| {
            fb.clear(0xFF000000);
            zb.clear();
            sys.render(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(view),
                black_box(proj),
            );
        });
    });
}

criterion_group!(benches, bench_colored_particles_render);
criterion_main!(benches);
