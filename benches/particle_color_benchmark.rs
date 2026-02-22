use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::particles::ParticleSystem;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use abrash::utils::XorShift32;

fn bench_particles_color_render(c: &mut Criterion) {
    c.bench_function("particles_render_50k_colored", |b| {
        let width = 800;
        let height = 600;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Create a texture for particles
        let texture = Texture::new(32, 32).unwrap();

        // Initialize particle system with capacity for 50k particles
        let mut sys = ParticleSystem::new(50000, texture);
        sys.emission_rate = 50000.0; // Emit all at once
        sys.start_life = 10.0; // Live long enough
        sys.spread = 10.0; // Spread out to cover screen

        // Simulate one frame to spawn particles
        sys.update(1.0);

        // Ensure we have particles
        assert!(sys.particles.len() >= 40000);

        // Assign random colors
        let mut rng = XorShift32::new(999);
        for p in &mut sys.particles {
            let r = (rng.next_f32() * 255.0) as u32;
            let g = (rng.next_f32() * 255.0) as u32;
            let b = (rng.next_f32() * 255.0) as u32;
            p.color = 0xFF000000 | (r << 16) | (g << 8) | b;
        }

        // Setup camera
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 20.0), // Camera back
            Vec3::new(0.0, 0.0, 0.0),  // Look at center
            Vec3::new(0.0, 1.0, 0.0),  // Up
        );
        let proj = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 100.0);

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

criterion_group!(benches, bench_particles_color_render);
criterion_main!(benches);
