use abrash::experimental::boids::BoidSystem;
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{Event, Window, WindowBackend};
use abrash::rasterizer::tile::TileRenderer;
use abrash::scene::{Camera, Scene, SceneObject};
use abrash::zbuffer::ZBuffer;
use std::sync::Arc;
use std::time::Instant;

fn pyramid(base_size: f32, height: f32) -> Mesh {
    let mut mesh = Mesh::new();
    let half_base = base_size / 2.0;

    // Define vertices
    // Base vertices (on XZ plane)
    mesh.vertices.push(Vec3::new(-half_base, 0.0, -half_base)); // 0
    mesh.vertices.push(Vec3::new(half_base, 0.0, -half_base)); // 1
    mesh.vertices.push(Vec3::new(half_base, 0.0, half_base)); // 2
    mesh.vertices.push(Vec3::new(-half_base, 0.0, half_base)); // 3
    // Tip vertex
    mesh.vertices.push(Vec3::new(0.0, 0.0, height)); // 4 (forward is Z+)

    // Define indices for the 4 sides
    mesh.indices.push([0, 4, 1]); // Top
    mesh.indices.push([1, 4, 2]); // Right
    mesh.indices.push([2, 4, 3]); // Bottom
    mesh.indices.push([3, 4, 0]); // Left

    // Base (two triangles)
    mesh.indices.push([0, 1, 2]);
    mesh.indices.push([0, 2, 3]);

    mesh
}

fn main() {
    let width = 160;
    let height = 80;

    let mut window = Window::new("Abrash - Boids Simulation Demo", width, height).unwrap();

    let mut fb = Framebuffer::new(width as u32, height as u32).unwrap();
    let mut zb = ZBuffer::new(width as u32, height as u32).unwrap();
    let mut renderer = TileRenderer::new(width as u32, height as u32);

    let eye = Vec3::new(0.0, 10.0, -30.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);

    let view = Mat4::look_at(eye, target, up);
    let proj = Mat4::perspective(1.047, width as f32 / height as f32, 0.1, 100.0);
    let camera = Camera::new(view, proj);
    let mut scene = Scene::new(camera);

    let pyramid_mesh = Arc::new(pyramid(1.0, 2.0));

    let boundary_size = 15.0;
    let mut boid_sys = BoidSystem::new(50, boundary_size);
    boid_sys.max_speed = 8.0; // speed up slightly

    let mut last_time = Instant::now();
    let mut running = true;

    while running && window.is_open() {
        let now = Instant::now();
        let dt = now.duration_since(last_time).as_secs_f32().min(0.05); // cap dt
        last_time = now;

        for event in window.poll_events() {
            if let Event::Close = event {
                running = false;
            }
        }

        boid_sys.update(dt);

        scene.objects.clear();
        for boid in &boid_sys.boids {
            let transform = boid.transform_matrix(0.5);
            scene.add_object(SceneObject::new(
                Arc::clone(&pyramid_mesh),
                transform,
                boid.color,
            ));
        }

        fb.clear(0xFF101010);
        zb.clear();

        scene.render(&mut renderer, &mut fb, &mut zb);
        window.blit_framebuffer(&fb);
    }
}
