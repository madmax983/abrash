//! Boids Flocking Simulation Demo
//!
//! A procedural crowd simulation based on Craig Reynolds' Boids algorithm,
//! rendered as a flock of 3D pyramids.

use abrash::experimental::boids::{Boid, Flock};
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::WindowBackend;
use abrash::platform::tui::TuiWindow;
use abrash::rasterizer::tile::TileRenderer;
use abrash::scene::{Camera, SceneObject, Scene};
use abrash::zbuffer::ZBuffer;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    let width = 120;
    let height = 60;

    let mut window = TuiWindow::new("Boids", width, height).unwrap();
    let mut fb = Framebuffer::new(width as u32, height as u32).unwrap();
    let mut zb = ZBuffer::new(width as u32, height as u32).unwrap();
    let mut renderer = TileRenderer::new(width as u32, height as u32);

    let num_boids = 50;
    let mut boids = Vec::with_capacity(num_boids);

    // Simple PRNG for boid initialization
    let mut seed: u32 = 42;
    let mut rand = || -> f32 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed as f32) / (std::u32::MAX as f32)
    };

    for _ in 0..num_boids {
        let x = rand() * 20.0 - 10.0;
        let y = rand() * 20.0 - 10.0;
        let z = rand() * 20.0 - 10.0;
        let vx = rand() * 2.0 - 1.0;
        let vy = rand() * 2.0 - 1.0;
        let vz = rand() * 2.0 - 1.0;

        boids.push(Boid {
            position: Vec3::new(x, y, z),
            velocity: Vec3::new(vx, vy, vz).fast_normalize(),
        });
    }

    let mut flock = Flock::new(boids);

    // Setup simple pyramid mesh for boids
    let mut pyramid = Mesh::with_capacity(5, 6);
    // Tip points forward
    pyramid.vertices.push(Vec3::new(0.0, 0.0, 0.5)); // Tip
    pyramid.vertices.push(Vec3::new(-0.2, -0.2, -0.5)); // Base bl
    pyramid.vertices.push(Vec3::new(0.2, -0.2, -0.5)); // Base br
    pyramid.vertices.push(Vec3::new(0.2, 0.2, -0.5)); // Base tr
    pyramid.vertices.push(Vec3::new(-0.2, 0.2, -0.5)); // Base tl

    // Front face
    pyramid.indices.push([0, 1, 2]);
    // Right face
    pyramid.indices.push([0, 2, 3]);
    // Back face (top)
    pyramid.indices.push([0, 3, 4]);
    // Left face
    pyramid.indices.push([0, 4, 1]);
    // Base
    pyramid.indices.push([1, 4, 3]);
    pyramid.indices.push([1, 3, 2]);

    let pyramid_arc = Arc::new(pyramid);

    let mut last_time = Instant::now();
    let mut rotation_angle = 0.0;

    while window.is_open() {
        let _events = window.poll_events();

        let now = Instant::now();
        let dt = now.duration_since(last_time).as_secs_f32();
        last_time = now;

        // Cap dt to prevent physics explosion if paused
        let dt = dt.min(0.05);

        flock.update(dt);

        fb.clear(0xFF000000); // Black background
        zb.clear();

        // Orbit camera around the flock
        rotation_angle += dt * 0.2;
        let cam_radius = 30.0;
        let eye = Vec3::new(
            rotation_angle.cos() * cam_radius,
            15.0,
            rotation_angle.sin() * cam_radius,
        );
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);

        let mut scene = Scene::new(Camera::new(
            Mat4::look_at(eye, target, up),
            Mat4::perspective(
                1.047,
                width as f32 / height as f32,
                0.1,
                100.0,
            )
        ));

        // Add each boid to the scene
        for (i, boid) in flock.boids.iter().enumerate() {
            let pos = boid.position;

            // Build orientation matrix from basis vectors
            let (fwd, up_vec, right) = boid.basis_vectors();
            let mut rot_matrix = Mat4::identity();
            rot_matrix.m[0][0] = right.x; rot_matrix.m[0][1] = right.y; rot_matrix.m[0][2] = right.z;
            rot_matrix.m[1][0] = up_vec.x; rot_matrix.m[1][1] = up_vec.y; rot_matrix.m[1][2] = up_vec.z;
            rot_matrix.m[2][0] = fwd.x; rot_matrix.m[2][1] = fwd.y; rot_matrix.m[2][2] = fwd.z;

            let translation = Mat4::translation(pos.x, pos.y, pos.z);
            let transform = rot_matrix * translation;

            // Generate a color based on the boid index
            let color = 0xFF000000 | ((i as u32 * 5) % 255) << 16 | (200 << 8) | ((i as u32 * 10) % 255);

            scene.objects.push(SceneObject::new(
                pyramid_arc.clone(),
                transform,
                color
            ));
        }

        scene.render(&mut renderer, &mut fb, &mut zb);

        window.blit_framebuffer(&fb);
    }
}
