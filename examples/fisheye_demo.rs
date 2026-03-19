//! Fisheye Filter Demo
//!
//! A visual demonstration of the Fisheye post-processing effect over a
//! 3D scene.

use abrash::platform::Event;
use abrash_core::math::{Mat4, Vec3, Vec4};
use abrash_render::experimental::fisheye::{apply_fisheye, FisheyeConfig};
use abrash_render::experimental::procedural_mesh;
use abrash_render::framebuffer::Framebuffer;
use abrash_render::mesh::Mesh;
use abrash_render::rasterizer::Rasterizer;
use abrash_render::RenderConfig;
use std::time::Instant;

fn main() {
    let width = 800;
    let height = 600;

    let mut platform = abrash::platform::get_platform_system(width, height, "Fisheye Demo")
        .expect("Failed to create window");

    let mut fb = Framebuffer::new(width, height).unwrap();
    let config = RenderConfig {
        width,
        height,
        enable_zbuffer: true,
        enable_simd: true,
        ..Default::default()
    };
    let mut rasterizer = Rasterizer::new(&config);

    // Create a checkered floor grid and a few cubes to demonstrate the fisheye distortion clearly
    let mut scene_meshes: Vec<Mesh> = Vec::new();

    // Create a 20x20 checkered floor using procedural_mesh
    let plane_mesh = procedural_mesh::create_plane(20.0, 20);
    scene_meshes.push(plane_mesh);

    // Add some cubes for depth
    let mut cube1 = Mesh::cube(1.0);
    for v in &mut cube1.vertices {
        let p = v.position;
        let translated = Mat4::translation(2.0, 1.0, 2.0) * Vec4::new(p.x, p.y, p.z, 1.0);
        v.position = Vec3::new(translated.x, translated.y, translated.z);
        v.color = 0xFF_FF_00_00; // Red cube
    }
    scene_meshes.push(cube1);

    let mut cube2 = Mesh::cube(1.0);
    for v in &mut cube2.vertices {
        let p = v.position;
        let translated = Mat4::translation(-2.0, 1.0, -2.0) * Vec4::new(p.x, p.y, p.z, 1.0);
        v.position = Vec3::new(translated.x, translated.y, translated.z);
        v.color = 0xFF_00_FF_00; // Green cube
    }
    scene_meshes.push(cube2);

    let mut cube3 = Mesh::cube(1.0);
    for v in &mut cube3.vertices {
        let p = v.position;
        let translated = Mat4::translation(-2.0, 1.0, 2.0) * Vec4::new(p.x, p.y, p.z, 1.0);
        v.position = Vec3::new(translated.x, translated.y, translated.z);
        v.color = 0xFF_00_00_FF; // Blue cube
    }
    scene_meshes.push(cube3);

    let mut cube4 = Mesh::cube(1.0);
    for v in &mut cube4.vertices {
        let p = v.position;
        let translated = Mat4::translation(2.0, 1.0, -2.0) * Vec4::new(p.x, p.y, p.z, 1.0);
        v.position = Vec3::new(translated.x, translated.y, translated.z);
        v.color = 0xFF_FF_FF_00; // Yellow cube
    }
    scene_meshes.push(cube4);

    // Camera parameters
    let camera_pos = Vec3::new(0.0, 6.0, 8.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let view = Mat4::look_at(camera_pos, target, up);

    let aspect = width as f32 / height as f32;
    let projection = Mat4::perspective(std::f32::consts::PI / 3.0, aspect, 0.1, 100.0);

    let mut fisheye_config = FisheyeConfig {
        center_x: 0.5,
        center_y: 0.5,
        strength: 2.0,
    };

    let start_time = Instant::now();

    'running: loop {
        while let Some(event) = platform.poll_events() {
            match event {
                Event::Close => break 'running,
                _ => {}
            }
        }

        let elapsed = start_time.elapsed().as_secs_f32();

        // Animate the fisheye effect:
        // Pulsate strength and move the center slightly over time
        fisheye_config.strength = 1.0 + (elapsed * 2.0).sin() * 0.5 + 0.5; // Pulsates between 1.0 and 2.0
        fisheye_config.center_x = 0.5 + (elapsed * 1.5).cos() * 0.2;
        fisheye_config.center_y = 0.5 + (elapsed * 1.1).sin() * 0.2;

        fb.clear(0xFF_22_22_22); // Dark gray background
        rasterizer.clear_zbuffer();

        // Slowly rotate the scene
        let model = Mat4::rotation_y(elapsed * 0.5);

        let mvp = projection * view * model;

        // Render all meshes
        for mesh in &scene_meshes {
            rasterizer.draw_mesh(&mut fb, mesh, &mvp);
        }

        // Apply post-processing
        apply_fisheye(&mut fb, &fisheye_config);

        platform.blit_framebuffer(fb.as_slice());
    }
}
