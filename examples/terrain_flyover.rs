use abrash::experimental::terrain::generate_terrain;
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::platform::{Window, WindowBackend};
use abrash::rasterizer::fill_triangle_gouraud;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF87_CEEB; // Sky Blue

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Abrash - Terrain Flyover", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;
    let mut timestep = FixedTimestep::new(60);

    // Generate Terrain
    // 64x64 grid, 100.0 physical size, 20.0 max height
    let terrain = generate_terrain(64, 64, 100.0, 20.0, 12345);

    // Pre-calculate colors for vertices
    // We store them as Vec3 (R, G, B) in 0.0..1.0 range
    let sun_dir = Vec3::new(0.5, 1.0, 0.5).normalize();
    let ambient = 0.3f32;

    let mut vertex_colors = Vec::with_capacity(terrain.mesh.vertices.len());
    for (i, v) in terrain.mesh.vertices.iter().enumerate() {
        let normal = terrain.normals[i];

        // Base color based on height
        let h = v.y / 20.0; // Normalized height roughly 0..1
        let base_color = if h < 0.1 {
            Vec3::new(0.0, 0.2, 0.8) // Water
        } else if h < 0.15 {
            Vec3::new(0.8, 0.7, 0.4) // Sand
        } else if h < 0.6 {
            Vec3::new(0.1, 0.5, 0.1) // Grass
        } else if h < 0.85 {
            Vec3::new(0.4, 0.4, 0.4) // Rock
        } else {
            Vec3::new(0.9, 0.9, 0.9) // Snow
        };

        // Apply lighting
        let diffuse = normal.dot(sun_dir).max(0.0);
        let light = (ambient + diffuse).min(1.0);

        vertex_colors.push(base_color * light);
    }

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 200.0);

    let mut time: f32 = 0.0;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            time += 0.005;
        }

        framebuffer.clear(BACKGROUND);
        zbuffer.clear();

        // Orbit camera
        let radius = 60.0;
        let cam_x = time.cos() * radius;
        let cam_z = time.sin() * radius;
        let cam_y = 30.0;

        let view = Mat4::look_at(
            Vec3::new(cam_x, cam_y, cam_z), // eye
            Vec3::new(0.0, 0.0, 0.0),       // target
            Vec3::new(0.0, 1.0, 0.0),       // up
        );

        let mvp = projection * view; // Model is identity

        // Render terrain
        // Iterate over indices
        for tri in terrain.mesh.indices.iter() {
            let i0 = tri[0];
            let i1 = tri[1];
            let i2 = tri[2];

            let v0 = terrain.mesh.vertices[i0];
            let v1 = terrain.mesh.vertices[i1];
            let v2 = terrain.mesh.vertices[i2];

            let c0 = vertex_colors[i0];
            let c1 = vertex_colors[i1];
            let c2 = vertex_colors[i2];

            // Transform vertices
            let (clip0, w0) = mvp.transform_point(v0);
            let (clip1, w1) = mvp.transform_point(v1);
            let (clip2, w2) = mvp.transform_point(v2);

            // Simple clipping check (behind camera)
            if w0 < 0.1 && w1 < 0.1 && w2 < 0.1 {
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
