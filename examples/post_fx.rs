//! Abrash Graphics Demo - Post-Processing FX
//!
//! Demonstrates retro post-processing effects.

use abrash::experimental::post_process::{
    ChromaticAberration, Grayscale, Invert, Pipeline, PostProcess, Scanlines,
};
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{Window, WindowBackend};
use abrash::rasterizer::fill_triangle_lit;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

// Face colors for the cube
const FACE_COLORS: [Vec3; 6] = [
    Vec3 {
        x: 0.90,
        y: 0.30,
        z: 0.24,
    }, // Red
    Vec3 {
        x: 0.18,
        y: 0.80,
        z: 0.44,
    }, // Green
    Vec3 {
        x: 0.20,
        y: 0.60,
        z: 0.86,
    }, // Blue
    Vec3 {
        x: 0.95,
        y: 0.61,
        z: 0.07,
    }, // Orange
    Vec3 {
        x: 0.61,
        y: 0.35,
        z: 0.71,
    }, // Purple
    Vec3 {
        x: 0.10,
        y: 0.74,
        z: 0.61,
    }, // Teal
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Abrash - Post FX Demo", WIDTH, HEIGHT)?;
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
    let ambient_color = Vec3::new(0.15, 0.15, 0.15);
    let sun_dir = Vec3::new(-0.5, -1.0, -0.3).normalize();
    let sun_color = Vec3::new(1.0, 0.95, 0.9);

    let mut timestep = FixedTimestep::new(60);
    let mut angle_y = 0.0f32;
    let mut angle_x = 0.0f32;

    // FX State
    let mut frame_count = 0;
    let mut fx_index = 0;
    let fx_names = [
        "None",
        "Grayscale",
        "Scanlines",
        "Chromatic Aberration",
        "Invert",
        "Combo: Retro TV",
    ];

    println!("Welcome to Nova's Post-FX Demo!");
    println!("Cycling effects every 120 frames (approx 2 seconds)...");

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle_y += 0.02;
            angle_x += 0.008;
            frame_count += 1;

            if frame_count % 120 == 0 {
                fx_index = (fx_index + 1) % fx_names.len();
                println!(">> Active Effect: {}", fx_names[fx_index]);
            }
        }

        // Clear buffers
        framebuffer.clear(0xFF1A_1A2E); // Dark blue background
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
            let face_color = FACE_COLORS[face_idx / 2];

            // Render with lighting
            fill_triangle_lit(
                &mut framebuffer,
                &mut zbuffer,
                v0,
                v1,
                v2,
                world_normal,
                face_color,
                ambient_color,
                sun_dir,
                sun_color,
            );
        }

        // Apply Post-Process Effects
        match fx_index {
            0 => {} // None
            1 => Grayscale.apply(&mut framebuffer),
            2 => Scanlines::new(0.3, 2).apply(&mut framebuffer),
            3 => ChromaticAberration::new(3, -3).apply(&mut framebuffer),
            4 => Invert.apply(&mut framebuffer),
            5 => {
                // Combo: Retro TV (Scanlines + Chromatic Aberration)
                let pipeline = Pipeline::new()
                    .add(Box::new(Scanlines::new(0.2, 2)))
                    .add(Box::new(ChromaticAberration::new(2, -2)));
                pipeline.apply(&mut framebuffer);
            }
            _ => {}
        }

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
