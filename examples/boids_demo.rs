#![allow(warnings)]

#[cfg(feature = "nova")]
use abrash::experimental::boids::{Boid, BoidSystem};
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::tile::TileRenderer;
use abrash::scene::{Camera, Scene, SceneObject};
use abrash::zbuffer::ZBuffer;
use std::sync::Arc;
use std::time::Instant;

#[cfg(not(feature = "nova"))]
fn main() {
    println!("The 'nova' feature is required to run the boids demo.");
}

#[cfg(feature = "nova")]
fn main() {
    println!("🌟 Nova: Boids Simulation Demo");
    println!("Rendering 1 frame to standard out (or visually, if platform supports)");

    let width = 800;
    let height = 600;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);

    // Setup Camera
    let eye = Vec3::new(0.0, 30.0, -50.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let view = Mat4::look_at(eye, target, up);
    let proj = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 1000.0);
    let camera = Camera::new(view, proj);

    let mut scene = Scene::new(camera);

    // Initialize Boids
    let mut system = BoidSystem::new(100, 20.0);

    // Create a pyramid mesh for the boids
    // A simple tetrahedron pointing along the Z axis
    let mut pyramid_mesh = Mesh::new();
    pyramid_mesh.vertices = vec![
        Vec3::new(0.0, 0.0, 1.0),    // Front tip
        Vec3::new(-0.5, -0.5, -1.0), // Bottom Left
        Vec3::new(0.5, -0.5, -1.0),  // Bottom Right
        Vec3::new(0.0, 0.5, -1.0),   // Top
    ];
    pyramid_mesh.indices = vec![
        [0, 1, 2], // Bottom Face
        [0, 2, 3], // Right Face
        [0, 3, 1], // Left Face
        [3, 2, 1], // Back Face
    ];
    // Default normals, uvs, tangents
    for _ in 0..4 {
        pyramid_mesh.normals.push(Vec3::new(0.0, 1.0, 0.0));
        pyramid_mesh.tangents.push(abrash::math::Vec4::default());
        pyramid_mesh.uvs.push(abrash::math::Vec2::new(0.0, 0.0));
    }
    let shared_mesh = Arc::new(pyramid_mesh);

    // Initial warm-up update
    system.update(1.0 / 60.0);

    // Build scene objects for each boid
    scene.objects.clear();
    for boid in &system.boids {
        // Calculate orientation matrix
        // We want the pyramid (pointing +Z locally) to point along the velocity
        let forward = boid.velocity.normalize();

        // Pick an arbitrary up vector, handle parallel cases
        let mut arb_up = Vec3::new(0.0, 1.0, 0.0);
        if forward.y.abs() > 0.99 {
            arb_up = Vec3::new(1.0, 0.0, 0.0);
        }

        let right = arb_up.cross(forward).normalize();
        let up_vec = forward.cross(right).normalize();

        // Create Rotation matrix (column major basis vectors, then translated)
        // Since Mat4 is row-major in this engine, we construct it accordingly
        // R * T means first translation, then rotation? Actually in this engine
        // it's typically v * M, so we want the matrix to be:
        // [ right.x, right.y, right.z, 0 ]
        // [ up.x,    up.y,    up.z,    0 ]
        // [ fwd.x,   fwd.y,   fwd.z,   0 ]
        // [ pos.x,   pos.y,   pos.z,   1 ]

        let transform = Mat4 {
            m: [
                [right.x, right.y, right.z, 0.0],
                [up_vec.x, up_vec.y, up_vec.z, 0.0],
                [forward.x, forward.y, forward.z, 0.0],
                [boid.position.x, boid.position.y, boid.position.z, 1.0],
            ],
        };

        let color = 0xFF00FF00; // Green boids
        scene.add_object(SceneObject::new(shared_mesh.clone(), transform, color));
    }

    // Render Scene
    fb.clear(0xFF000000); // Black background
    zb.clear();

    let start = Instant::now();
    scene.render(&mut renderer, &mut fb, &mut zb);
    let duration = start.elapsed();

    println!("Rendered Boids frame in {:?}", duration);
    println!("Demo complete.");
}