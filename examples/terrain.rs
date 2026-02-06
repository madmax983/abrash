use abrash::experimental::terrain::TerrainGenerator;
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::platform::{Window, WindowBackend};
use abrash::rasterizer::fill_triangle_gouraud;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF87_CEEB; // Sky blue

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Abrash - Procedural Terrain", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;
    let mut timestep = FixedTimestep::new(60);

    // Generate terrain
    // 50x50 grid, scaled by 1.0
    let generator = TerrainGenerator::new(1337, 8.0);
    let mesh = generator.generate_mesh(40, 40, 1.0);

    // Pre-calculate colors based on height
    let colors: Vec<Vec3> = mesh
        .vertices
        .iter()
        .map(|v| {
            let h = v.y;
            if h > 4.0 {
                Vec3::new(1.0, 1.0, 1.0) // Snow
            } else if h > 1.5 {
                Vec3::new(0.5, 0.5, 0.5) // Rock
            } else if h > 0.0 {
                Vec3::new(0.1, 0.8, 0.1) // Grass
            } else {
                Vec3::new(0.0, 0.0, 0.8) // Water (if we had negative height)
            }
        })
        .collect();

    let mut camera_angle_y: f32 = 0.0;
    let camera_dist = 40.0;
    let camera_height = 25.0;

    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            camera_angle_y += 0.5 * timestep.dt();
        }

        framebuffer.clear(BACKGROUND);
        zbuffer.clear();

        // Orbit camera
        let cam_x = camera_angle_y.sin() * camera_dist;
        let cam_z = camera_angle_y.cos() * camera_dist;
        let eye = Vec3::new(cam_x, camera_height, cam_z);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);

        let view = Mat4::look_at(eye, target, up);
        let mvp = projection * view; // Model is identity

        for tri_indices in &mesh.indices {
            let idx0 = tri_indices[0];
            let idx1 = tri_indices[1];
            let idx2 = tri_indices[2];

            let v0 = mesh.vertices[idx0];
            let v1 = mesh.vertices[idx1];
            let v2 = mesh.vertices[idx2];

            let c0 = colors[idx0];
            let c1 = colors[idx1];
            let c2 = colors[idx2];

            // Transform
            let (clip0, w0) = mvp.transform_point(v0);
            let (clip1, w1) = mvp.transform_point(v1);
            let (clip2, w2) = mvp.transform_point(v2);

            // Simple culling
            if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                continue;
            }

            fill_triangle_gouraud(
                &mut framebuffer,
                &mut zbuffer,
                ((clip0, w0), c0),
                ((clip1, w1), c1),
                ((clip2, w2), c2),
            );
        }

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
