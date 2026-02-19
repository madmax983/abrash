//! Abrash Graphics Demo - SSAO
//!
//! Demonstrates Screen-Space Ambient Occlusion.
//! Renders a scene with multiple objects and a floor to show contact shadows.
//!
//! Use the --ssao flag to control initial state (default: on).
//! SSAO toggles every 3 seconds for comparison.

use abrash::framebuffer::Framebuffer;
use abrash::post_process::apply_ssao;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{Window, WindowBackend};
use abrash::rasterizer::fill_triangle_lit;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn create_floor() -> Mesh {
    let mut mesh = Mesh::new();
    let size = 10.0;
    let h = size / 2.0;

    // 4 vertices
    mesh.vertices.push(Vec3::new(-h, 0.0, h)); // 0
    mesh.vertices.push(Vec3::new(h, 0.0, h)); // 1
    mesh.vertices.push(Vec3::new(h, 0.0, -h)); // 2
    mesh.vertices.push(Vec3::new(-h, 0.0, -h)); // 3

    // 2 triangles (CCW winding)
    // 0 -> 1 -> 2 is CW from top.
    // We want CCW (Counter-Clockwise) to be visible from top.
    // 2 -> 1 -> 0
    // 3 -> 2 -> 0
    mesh.indices.push([2, 1, 0]);
    mesh.indices.push([3, 2, 0]);

    mesh
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Abrash - SSAO Demo", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT).unwrap();

    let cube = Mesh::cube(1.0);
    let floor = create_floor();

    let cube_normals = cube.compute_face_normals();
    let floor_normals = floor.compute_face_normals();

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 3.0, 5.0), // Eye
        Vec3::new(0.0, 0.0, 0.0), // Target
        Vec3::new(0.0, 1.0, 0.0), // Up
    );

    // Lighting setup
    let ambient_color = Vec3::new(0.2, 0.2, 0.2);
    let sun_dir = Vec3::new(-0.5, -1.0, -0.3).normalize();
    let sun_color = Vec3::new(0.8, 0.8, 0.8);
    let floor_color = Vec3::new(0.6, 0.6, 0.6);
    let cube_color = Vec3::new(0.9, 0.4, 0.3);

    let mut timestep = FixedTimestep::new(60);
    let mut time = 0.0f32;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            time += 0.016;
        }

        // Toggle SSAO every 3 seconds
        let ssao_enabled = (time % 6.0) < 3.0;

        // Clear buffers
        framebuffer.clear(0xFF1A_1A2E); // Dark blue background
        zbuffer.clear();

        // Render Floor
        // Floor is at Y = -0.5 (below cubes)
        let model_floor = Mat4::translation(0.0, -0.5, 0.0);
        let mvp_floor = projection * (view * model_floor);
        let normal_mat_floor = model_floor; // No rotation/scale, so normal transform is same

        for (face_idx, tri) in floor.indices.iter().enumerate() {
            let [i0, i1, i2] = *tri;
            let v0 = mvp_floor.transform_point(floor.vertices[i0]);
            let v1 = mvp_floor.transform_point(floor.vertices[i1]);
            let v2 = mvp_floor.transform_point(floor.vertices[i2]);
            let normal = normal_mat_floor.transform_normal(floor_normals[face_idx]);

            fill_triangle_lit(
                &mut framebuffer,
                &mut zbuffer,
                v0,
                v1,
                v2,
                normal,
                floor_color,
                ambient_color,
                sun_dir,
                sun_color,
            );
        }

        // Render Cubes
        // Position them so they touch the floor (y=-0.5).
        // Cube size 1.0 -> extends -0.5 to 0.5 in local.
        // We want bottom at -0.5.
        // If center is at Y=0, bottom is -0.5.
        // So cubes at Y=0 sit exactly on floor at Y=-0.5.
        let cube_positions = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(-1.2, 0.0, 0.5),
            Vec3::new(1.2, 0.0, -0.5),
            Vec3::new(0.0, 1.0, 0.0), // Stacked on center (bottom at 0.5, sits on top of center cube)
        ];

        for pos in cube_positions.iter() {
            let model = Mat4::translation(pos.x, pos.y, pos.z);
            let mvp = projection * (view * model);
            let normal_mat = model;

            for (face_idx, tri) in cube.indices.iter().enumerate() {
                let [i0, i1, i2] = *tri;
                let v0 = mvp.transform_point(cube.vertices[i0]);
                let v1 = mvp.transform_point(cube.vertices[i1]);
                let v2 = mvp.transform_point(cube.vertices[i2]);
                let normal = normal_mat.transform_normal(cube_normals[face_idx]);

                fill_triangle_lit(
                    &mut framebuffer,
                    &mut zbuffer,
                    v0,
                    v1,
                    v2,
                    normal,
                    cube_color,
                    ambient_color,
                    sun_dir,
                    sun_color,
                );
            }
        }

        // Apply SSAO
        if ssao_enabled {
            // radius 0.5, bias 0.025, intensity 2.0
            apply_ssao(&mut framebuffer, &zbuffer, &projection, 0.5, 0.025, 2.0);

            // Draw "SSAO ON" indicator (simple pixel block in top left)
            for y in 10..20 {
                for x in 10..20 {
                    framebuffer.set_pixel(x, y, 0xFF00FF00); // Green
                }
            }
        } else {
            // Draw "SSAO OFF" indicator (Red)
            for y in 10..20 {
                for x in 10..20 {
                    framebuffer.set_pixel(x, y, 0xFFFF0000); // Red
                }
            }
        }

        window.blit_framebuffer(&framebuffer);

        // Sleep a bit to limit CPU usage if window backend doesn't vsync
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    Ok(())
}
