//! SDF Fluid Simulation Example
//!
//! Demonstrates the interaction between particles and SDF geometry using SPH forces.

#![cfg_attr(not(feature = "nova"), allow(unused))]

#[cfg(feature = "nova")]
fn main() {
    use abrash::ascii::{AsciiCharset, AsciiConverter};
    use abrash::experimental::sdf::{SdfObject, SdfPrimitive, SdfScene, render_sdf};
    use abrash::experimental::sdf_particles::{apply_sph_forces, resolve_sdf_collisions};
    use abrash::framebuffer::Framebuffer;
    use abrash::math::{Mat4, Vec3};
    use abrash::particles::{Particle, ParticleSystem};
    use abrash::texture::Texture;
    use abrash::zbuffer::ZBuffer;

    // 1. Setup Framebuffer (Low res for ASCII)
    let width = 80;
    let height = 40;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // 2. Setup SDF Scene (Container)
    let mut scene = SdfScene::new();

    // Floor (Sphere)
    scene.add(SdfObject {
        primitive: SdfPrimitive::Sphere {
            radius: 10.0,
            center: Vec3::new(0.0, -12.0, 0.0), // Top at -2.0
        },
        color: 0xFF444444,
    });

    // Left Wall
    scene.add(SdfObject {
        primitive: SdfPrimitive::Box {
            size: Vec3::new(1.0, 5.0, 5.0),
            center: Vec3::new(-5.0, 0.0, 0.0),
        },
        color: 0xFF880000,
    });

    // Right Wall
    scene.add(SdfObject {
        primitive: SdfPrimitive::Box {
            size: Vec3::new(1.0, 5.0, 5.0),
            center: Vec3::new(5.0, 0.0, 0.0),
        },
        color: 0xFF880000,
    });

    // 3. Setup Particles
    let texture = Texture::new(2, 2).unwrap(); // Dummy texture
    let mut system = ParticleSystem::new(200, texture);
    system.emission_rate = 0.0;
    system.gravity = Vec3::new(0.0, -9.8, 0.0);

    // Spawn block of particles
    for x in -2..3 {
        for y in 0..10 {
            for z in -2..3 {
                system.particles.push(Particle::new(
                    Vec3::new(x as f32 * 0.6, y as f32 * 0.6 + 5.0, z as f32 * 0.6),
                    Vec3::new(0.0, 0.0, 0.0),
                    10.0,       // Long life
                    0.4,        // Size
                    0xFF00FFFF, // Cyan
                ));
            }
        }
    }

    // 4. Camera
    let eye = Vec3::new(0.0, 5.0, 15.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let view = Mat4::look_at(eye, target, up);
    let proj = Mat4::perspective(1.0, width as f32 / height as f32, 0.1, 100.0);

    println!("Simulating Fluid (SPH + SDF)...");

    // Simulation Loop
    for frame in 0..100 {
        let dt = 0.03; // Fixed time step

        // Physics
        system.update(dt);

        // SPH Forces
        apply_sph_forces(&mut system, 1.0, 0.5, 50.0, 2.0, dt);

        // Collisions
        resolve_sdf_collisions(&mut system, &scene, 0.5, 0.1);

        // Render frames 0, 50, 99
        if frame == 0 || frame == 50 || frame == 99 {
            fb.clear(0xFF000000);
            zb.clear();

            // Render SDF
            render_sdf(&mut fb, &mut zb, &scene, &view, &proj, eye);

            // Render Particles
            system.render(&mut fb, &mut zb, view, proj);

            // ASCII Output
            let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
            println!("Frame {}:\n{}", frame, converter.to_string());
        }
    }
}

#[cfg(not(feature = "nova"))]
fn main() {
    println!("This example requires the 'nova' feature.");
    println!("Run with: cargo run --example sdf_fluid --features nova");
}
