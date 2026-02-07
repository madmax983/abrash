use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::obj_loader::load_obj;
use abrash::platform::{Window, WindowBackend};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF101010;

// Embed a simple spaceship-like OBJ
const SPACESHIP_OBJ: &str = r#"
# Simple Spacerocket
v 0.0 1.5 0.0
v 0.5 -0.5 0.5
v -0.5 -0.5 0.5
v -0.5 -0.5 -0.5
v 0.5 -0.5 -0.5
v 0.0 -0.8 0.0
# Top pyramid
f 1 2 3
f 1 3 4
f 1 4 5
f 1 5 2
# Bottom inverted pyramid (engine)
f 6 3 2
f 6 4 3
f 6 5 4
f 6 2 5
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Abrash - OBJ Viewer", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;
    let mut timestep = FixedTimestep::new(60);

    // Load the mesh
    let mesh = load_obj(SPACESHIP_OBJ).map_err(|e| format!("Failed to load OBJ: {}", e))?;

    // Compute normals for flat shading logic (simple color variation)
    let normals = mesh.compute_face_normals();

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 1.0, 3.0), // eye
        Vec3::new(0.0, 0.0, 0.0), // target
        Vec3::new(0.0, 1.0, 0.0), // up
    );

    let mut angle_y: f32 = 0.0;

    // Simple palette based on normal direction
    let base_color = Vec3::new(0.4, 0.6, 1.0); // Light blue

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle_y += 1.0 * timestep.dt();
        }

        framebuffer.clear(BACKGROUND);
        zbuffer.clear();

        // Model matrix (rotation)
        let model = Mat4::rotation_y(angle_y);

        // MVP matrix
        let mvp = projection * (view * model);

        // Rotation matrix for normals (upper 3x3 of model)
        let normal_mat = model; // For rotation only, this is fine

        // Transform and render each triangle
        for (i, tri_indices) in mesh.indices.iter().enumerate() {
            let v0 = mesh.vertices[tri_indices[0]];
            let v1 = mesh.vertices[tri_indices[1]];
            let v2 = mesh.vertices[tri_indices[2]];

            // Transform vertices
            let (clip0, w0) = mvp.transform_point(v0);
            let (clip1, w1) = mvp.transform_point(v1);
            let (clip2, w2) = mvp.transform_point(v2);

            // Simple backface culling
            if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                continue;
            }

            // Calculate color based on normal
            // Since Mesh doesn't store normals per vertex, we use the face normal
            let normal = if i < normals.len() {
                normals[i]
            } else {
                Vec3::new(0.0, 1.0, 0.0)
            };

            // Rotate normal
            let world_normal = normal_mat.transform_normal(normal);

            // Simple directional light from top-right
            let light_dir = Vec3::new(0.5, 1.0, 0.5).normalize();
            let diffuse = world_normal.dot(light_dir).max(0.2);

            let color_vec = base_color * diffuse;

            // Pack color (ARGB)
            let r = (color_vec.x * 255.0).min(255.0) as u32;
            let g = (color_vec.y * 255.0).min(255.0) as u32;
            let b = (color_vec.z * 255.0).min(255.0) as u32;
            let color = 0xFF000000 | (r << 16) | (g << 8) | b;

            fill_triangle_3d(
                &mut framebuffer,
                &mut zbuffer,
                (clip0, w0),
                (clip1, w1),
                (clip2, w2),
                color,
            );
        }

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
