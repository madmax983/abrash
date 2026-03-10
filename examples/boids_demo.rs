use abrash::experimental::boids::{Boid, BoidsSimulation};
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{Event, WindowBackend};
#[cfg(feature = "backend-win32")]
use abrash::platform::Window as Win32Window;
#[cfg(feature = "backend-tui")]
use abrash::platform::Window as TuiWindow;
use abrash::rasterizer::tile::TileRenderer;
use abrash::scene::{Camera, Scene, SceneObject};
use std::time::Instant;
use std::sync::Arc;
use abrash::zbuffer::ZBuffer;
use rand::Rng;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const NUM_BOIDS: usize = 300;

fn create_pyramid_mesh() -> Mesh {
    // A simple pyramid pointing along the +Z axis.
    let vertices = vec![
        Vec3::new(0.0, 0.0, 1.5),     // Tip (Forward)
        Vec3::new(-0.5, -0.5, -0.5),  // Base Bottom Left
        Vec3::new(0.5, -0.5, -0.5),   // Base Bottom Right
        Vec3::new(0.5, 0.5, -0.5),    // Base Top Right
        Vec3::new(-0.5, 0.5, -0.5),   // Base Top Left
    ];

    let indices = vec![
        // Base
        [2, 1, 4],
        [4, 3, 2],
        // Sides
        [0, 1, 2],
        [0, 2, 3],
        [0, 3, 4],
        [0, 4, 1],
    ];

    let mut mesh = Mesh::new();
    mesh.vertices = vertices;
    mesh.indices = indices;
    mesh.normals = mesh.compute_face_normals();
    mesh
}

fn create_boids() -> Vec<Boid> {
    let mut rng = rand::thread_rng();
    let mut boids = Vec::with_capacity(NUM_BOIDS);
    for _ in 0..NUM_BOIDS {
        boids.push(Boid::new(
            Vec3::new(
                rng.gen_range(-20.0..20.0),
                rng.gen_range(-20.0..20.0),
                rng.gen_range(-20.0..20.0),
            ),
            Vec3::new(
                rng.gen_range(-5.0..5.0),
                rng.gen_range(-5.0..5.0),
                rng.gen_range(-5.0..5.0),
            ),
        ));
    }
    boids
}

fn main() {
    #[cfg(feature = "backend-win32")]
    let mut window = Win32Window::new("🌟 Nova: Boids Flock", WIDTH, HEIGHT).unwrap();
    #[cfg(feature = "backend-tui")]
    let mut window = TuiWindow::new("🌟 Nova: Boids Flock", WIDTH, HEIGHT).unwrap();
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT).unwrap();
    let mut renderer = TileRenderer::new(WIDTH, HEIGHT);
    let mut last_frame = Instant::now();

    let pyramid_mesh = Arc::new(create_pyramid_mesh());
    let initial_boids = create_boids();
    let mut sim = BoidsSimulation::new(initial_boids);
    sim.bounds = 30.0;
    sim.max_speed = 15.0;

    let projection = Mat4::perspective(
        std::f32::consts::PI / 3.0,
        WIDTH as f32 / HEIGHT as f32,
        0.1,
        200.0,
    );
    let camera_pos = Vec3::new(0.0, 20.0, 60.0);
    let view = Mat4::look_at(camera_pos, Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    let camera = Camera::new(view, projection);

    let mut scene = Scene::new(camera);

    // Populate scene with dummy objects (one for each boid)
    for _ in 0..NUM_BOIDS {
        let obj = SceneObject::new(
            pyramid_mesh.clone(),
            Mat4::identity(),
            0xFF_00_AA_FF, // Bright blueish cyan
        );
        scene.add_object(obj);
    }

    println!("Boids Simulation Started");
    println!("Controls:");
    println!("  Space : Scatter flock");
    println!("  ESC   : Exit");

    while window.is_open() {
        let now = Instant::now();
        let dt = now.duration_since(last_frame).as_secs_f32();
        last_frame = now;

        for event in window.poll_events() {
            if let Event::Close = event {
                return;
            }
        }

        sim.step(dt);

        // Update scene objects to match boids
        for (i, boid) in sim.boids.iter().enumerate() {
            let (forward, up, right) = BoidsSimulation::compute_basis(boid.velocity);

            // Construct an orientation matrix using the basis vectors
            // (Right, Up, -Forward) for a standard Right-Handed look-at
            let mut transform = Mat4::identity();
            transform.m[0][0] = right.x;
            transform.m[1][0] = right.y;
            transform.m[2][0] = right.z;

            transform.m[0][1] = up.x;
            transform.m[1][1] = up.y;
            transform.m[2][1] = up.z;

            transform.m[0][2] = forward.x;
            transform.m[1][2] = forward.y;
            transform.m[2][2] = forward.z;

            // Apply translation
            let translation = Mat4::translation(boid.position.x, boid.position.y, boid.position.z);
            let final_transform = transform * translation;

            scene.objects[i].transform = final_transform;
        }

        framebuffer.clear(0xFF_11_11_22); // Dark blueish background
        zbuffer.clear();

        scene.render(&mut renderer, &mut framebuffer, &mut zbuffer);

        window.blit_framebuffer(&framebuffer);
    }
}
