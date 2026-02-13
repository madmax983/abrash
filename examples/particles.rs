use abrash::experimental::particles::ParticleSystem;
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::platform::{Window, WindowBackend};
use abrash::texture::Texture;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF10_1010;

fn create_particle_texture() -> Texture {
    let size = 32;
    let mut tex = Texture::new(size, size).unwrap();
    let center = size as f32 / 2.0;
    let max_dist = center;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > max_dist {
                // Fully transparent
                // In Abrash engine (currently), 0 is skipped (transparent),
                // but 254 is also transparent in blending path.
                tex.set_pixel(x as u32, y as u32, 0x0000_0000);
            } else {
                // Smooth falloff
                let t = dist / max_dist; // 0.0 (center) to 1.0 (edge)

                // We want Center = Opaque, Edge = Transparent.
                // Abrash blending quirk:
                // Alpha=1 -> Opaque (Src * 254 + Dest * 1)
                // Alpha=254 -> Transparent (Src * 1 + Dest * 254)
                // Alpha=255 -> Opaque (Overwrite)

                // So we map t (0..1) to Alpha (1..254)
                let alpha = 1.0 + t * 253.0;
                let alpha_u8 = alpha as u8;

                // Color: Orange Fire
                let r = 255;
                let g = ((1.0 - t) * 200.0) as u8; // Redder at edge
                let b = 0;

                let color = ((alpha_u8 as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b;
                tex.set_pixel(x as u32, y as u32, color);
            }
        }
    }
    tex.generate_mipmaps();
    tex
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Nova - Particle System", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;
    let mut timestep = FixedTimestep::new(60);

    let texture = create_particle_texture();
    let mut particles = ParticleSystem::new(1000, texture);
    particles.emission_rate = 50.0;
    particles.start_life = 2.0;
    particles.spread = 0.8;
    particles.start_size = 0.5;

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);

    // Rotate camera around center
    let mut angle: f32 = 0.0;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle += 0.5 * timestep.dt();
            particles.update(timestep.dt());
        }

        framebuffer.clear(BACKGROUND);
        zbuffer.clear();

        let eye = Vec3::new(angle.sin() * 5.0, 2.0, angle.cos() * 5.0);
        let target = Vec3::new(0.0, 1.0, 0.0); // Look slightly up
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);

        // Draw grid floor (optional, for reference)
        // ...

        particles.render(&mut framebuffer, &mut zbuffer, view, projection);

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
