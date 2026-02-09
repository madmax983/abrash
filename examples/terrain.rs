use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::platform::{Window, WindowBackend};
use abrash::rasterizer::fill_triangle_lit;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use abrash::experimental::terrain::generate_terrain_mesh;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF87_CEEB; // Sky Blue

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Abrash - Procedural Terrain", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;
    let mut timestep = FixedTimestep::new(60);

    // Generate terrain
    // Width: 40, Depth: 40, Scale: 0.15, Amplitude: 4.0
    let terrain = generate_terrain_mesh(40, 40, 0.15, 4.0, 1337);
    let normals = terrain.compute_face_normals();

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);

    // Initial camera position
    let camera_pos = Vec3::new(0.0, 10.0, -15.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);

    let mut angle: f32 = 0.0;

    // Lighting
    let light_dir = Vec3::new(0.5, -1.0, 0.5).normalize(); // Sunlight from top-right-front
    let light_color = Vec3::new(1.0, 0.95, 0.9); // Warm sunlight
    let ambient_color = Vec3::new(0.2, 0.2, 0.3); // Blueish shadows

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle += 0.2 * timestep.dt();
        }

        framebuffer.clear(BACKGROUND);
        zbuffer.clear();

        // Rotate camera around the terrain
        let rotation = Mat4::rotation_y(angle * 0.5);
        let cam_rotated = rotation.transform_point(camera_pos).0;

        let view = Mat4::look_at(cam_rotated, target, up);

        // MVP matrix
        let mvp = projection * view; // Model is Identity

        // Transform and render each triangle
        for (i, tri_indices) in terrain.indices.iter().enumerate() {
            let v0 = terrain.vertices[tri_indices[0]];
            let v1 = terrain.vertices[tri_indices[1]];
            let v2 = terrain.vertices[tri_indices[2]];

            // Calculate average height for coloring
            let avg_height = (v0.y + v1.y + v2.y) / 3.0;

            // Color based on height (Low Poly Style)
            let base_color = if avg_height < 1.0 {
                Vec3::new(0.2, 0.5, 0.2) // Grass
            } else if avg_height < 3.5 {
                Vec3::new(0.4, 0.4, 0.4) // Rock
            } else {
                Vec3::new(0.9, 0.9, 1.0) // Snow
            };

            // Transform vertices
            let (clip0, w0) = mvp.transform_point(v0);
            let (clip1, w1) = mvp.transform_point(v1);
            let (clip2, w2) = mvp.transform_point(v2);

            // Simple culling
            if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                continue;
            }

            // Get face normal
            let normal = normals[i];

            fill_triangle_lit(
                &mut framebuffer,
                &mut zbuffer,
                (clip0, w0),
                (clip1, w1),
                (clip2, w2),
                normal,
                base_color,
                ambient_color,
                light_dir,
                light_color,
            );
        }

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
