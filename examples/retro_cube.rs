//! Abrash Graphics Demo - Retro Cube
//!
//! Demonstrates the post-processing effects.

use abrash::framebuffer::Framebuffer;
use abrash::light::{AmbientLight, DirectionalLight, u32_to_color};
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::pipeline::fill_triangle_lit;
use abrash::platform::Window;
use abrash::post_process::{apply_grayscale, apply_scanlines};
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

// Face colors for the cube
const FACE_COLORS: [u32; 6] = [
    0xFFE74C3C, // Red
    0xFF2ECC71, // Green
    0xFF3498DB, // Blue
    0xFFF39C12, // Orange
    0xFF9B59B6, // Purple
    0xFF1ABC9C, // Teal
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Abrash - Retro Cube", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT).unwrap();
    let cube = Mesh::cube(1.5);
    let face_normals = cube.compute_face_normals();

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 2.0, 4.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    // Lighting setup
    let ambient = AmbientLight::new(Vec3::new(0.15, 0.15, 0.15));
    let sun = DirectionalLight::new(
        Vec3::new(-0.5, -1.0, -0.3),
        Vec3::new(1.0, 0.95, 0.9), // Warm sunlight
    );

    let mut timestep = FixedTimestep::new(60);
    let mut angle_y = 0.0f32;
    let mut angle_x = 0.0f32;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle_y += 0.02;
            angle_x += 0.008;
        }

        // Clear buffers
        framebuffer.clear(0xFF1A1A2E); // Dark blue background
        zbuffer.clear();

        // Build model matrix
        let model = Mat4::rotation_y(angle_y) * Mat4::rotation_x(angle_x);
        let mvp = projection * (view * model);

        // Render each face
        for (face_idx, tri_indices) in cube.indices.iter().enumerate() {
            let [i0, i1, i2] = *tri_indices;

            // Transform vertices
            let v0 = mvp.transform_point(cube.vertices[i0]);
            let v1 = mvp.transform_point(cube.vertices[i1]);
            let v2 = mvp.transform_point(cube.vertices[i2]);

            // Transform normal to world space (use model matrix only)
            let world_normal = model.transform_normal(face_normals[face_idx]);

            // Get face color (2 triangles per face)
            let face_color = u32_to_color(FACE_COLORS[face_idx / 2]);

            // Render with lighting
            fill_triangle_lit(
                &mut framebuffer,
                &mut zbuffer,
                v0,
                v1,
                v2,
                world_normal,
                face_color,
                &ambient,
                &sun,
            );
        }

        // Apply Retro Effects
        apply_grayscale(&mut framebuffer);
        apply_scanlines(&mut framebuffer, 0.25);

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
