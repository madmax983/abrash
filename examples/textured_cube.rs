//! Abrash Graphics Demo - Textured 3D Cube
//!
//! Demonstrates perspective-correct texture mapping on a rotating cube.

use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::Window;
use abrash::primitives::fill_triangle_textured;
use abrash::texture::Texture;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

/// Create a simple checkerboard texture
fn create_checkerboard(size: u32, square_size: u32) -> Texture {
    let mut texture = Texture::new(size, size);

    for y in 0..size {
        for x in 0..size {
            let checker_x = (x / square_size) % 2;
            let checker_y = (y / square_size) % 2;

            let color = if (checker_x + checker_y) % 2 == 0 {
                0xFFFFFFFF // White
            } else {
                0xFF222222 // Dark gray
            };

            texture.set_pixel(x as i32, y as i32, color);
        }
    }

    texture
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Abrash - Textured Cube", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT);
    let cube = Mesh::cube_textured(1.0); // Smaller cube for better framing
    let texture = create_checkerboard(64, 8);

    // Camera setup - narrower FOV and further back for better framing
    let projection = Mat4::perspective(PI / 4.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 1.5, 5.5), // Further back and lower camera
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    let mut timestep = FixedTimestep::new(60);
    let mut angle_y = 0.0f32;
    let mut angle_x = 0.0f32;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle_y += 0.015; // Slightly slower rotation
            angle_x += 0.006;
        }

        // Clear buffers
        framebuffer.clear(0xFF1A1A2E); // Dark blue background
        zbuffer.clear();

        // Build model matrix
        let model = Mat4::rotation_y(angle_y).mul(&Mat4::rotation_x(angle_x));
        let mvp = projection.mul(&view.mul(&model));

        // Render each triangle
        if let Some(ref uvs) = cube.uvs {
            for tri_indices in &cube.indices {
                let [i0, i1, i2] = *tri_indices;

                // Transform vertices
                let v0 = mvp.transform_point(cube.vertices[i0]);
                let v1 = mvp.transform_point(cube.vertices[i1]);
                let v2 = mvp.transform_point(cube.vertices[i2]);

                // Get UV coordinates
                let uv0 = uvs[i0];
                let uv1 = uvs[i1];
                let uv2 = uvs[i2];

                // Near-plane and backface culling
                let (clip0, w0) = v0;
                let (clip1, w1) = v1;
                let (clip2, w2) = v2;

                // Skip triangles behind or too close to camera
                if w0 <= 0.1 || w1 <= 0.1 || w2 <= 0.1 {
                    continue;
                }

                // Project to NDC for backface check
                let ndc0 = Vec3::new(clip0.x / w0, clip0.y / w0, clip0.z / w0);
                let ndc1 = Vec3::new(clip1.x / w1, clip1.y / w1, clip1.z / w1);
                let ndc2 = Vec3::new(clip2.x / w2, clip2.y / w2, clip2.z / w2);

                // Compute screen-space triangle area (2x area)
                let edge1 = Vec3::new(ndc1.x - ndc0.x, ndc1.y - ndc0.y, 0.0);
                let edge2 = Vec3::new(ndc2.x - ndc0.x, ndc2.y - ndc0.y, 0.0);
                let cross = edge1.cross(edge2);

                // Skip back-facing triangles (negative area)
                if cross.z <= 0.0 {
                    continue;
                }

                // Draw textured triangle
                fill_triangle_textured(
                    &mut framebuffer,
                    &mut zbuffer,
                    (v0, uv0),
                    (v1, uv1),
                    (v2, uv2),
                    &texture,
                );
            }
        }

        // Present frame
        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
