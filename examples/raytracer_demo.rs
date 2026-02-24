//! Abrash Raytracer Demo
//!
//! Demonstrates the experimental CPU raytracer with reflections and shadows.

use abrash::experimental::raytracer::RayTracer;
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{Window, WindowBackend};
use abrash::scene::{Camera, Scene, SceneObject};
use abrash::time::FixedTimestep;
use std::f32::consts::PI;
use std::sync::Arc;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 400;
const HEIGHT: u32 = 300;

fn print_banner() {
    println!("\n{}", "✨ Raytracer Demo".bold().cyan());
    println!("{}", "=====================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Resolution"),
            Cell::new(format!("{WIDTH}x{HEIGHT}")).fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Features"),
            Cell::new("Reflections, Soft Shadows (Simulated), Phong Shading").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();
    let mut window = Window::new("Abrash - Raytracer", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;

    let mut renderer = RayTracer::new();
    renderer.max_bounces = 3;
    renderer.background_color = 0xFF101015; // Deep dark blue/grey

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    // Position camera
    let eye = Vec3::new(0.0, 3.0, 6.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let view = Mat4::look_at(eye, target, up);

    let camera = Camera::new(view, projection);

    let mut scene = Scene::new(camera);

    // Create meshes
    let cube_mesh = Arc::new(Mesh::cube(1.0));

    // Floor (Flattened Cube)
    let floor_transform = Mat4::translation(0.0, -1.0, 0.0) * Mat4::scale(10.0, 0.1, 10.0);
    scene.add_object(SceneObject::new(
        cube_mesh.clone(),
        floor_transform,
        0xFF404040,
    )); // Grey

    // Center Cube (Red)
    let center_transform = Mat4::translation(0.0, 0.0, 0.0);
    scene.add_object(SceneObject::new(
        cube_mesh.clone(),
        center_transform,
        0xFFFF0000,
    ));

    // Left Cube (Green)
    let left_transform = Mat4::translation(-2.5, 0.0, -1.0) * Mat4::rotation_y(PI / 4.0);
    scene.add_object(SceneObject::new(
        cube_mesh.clone(),
        left_transform,
        0xFF00FF00,
    ));

    // Right Cube (Blue)
    let right_transform = Mat4::translation(2.5, 0.0, -1.0) * Mat4::rotation_y(-PI / 4.0);
    scene.add_object(SceneObject::new(
        cube_mesh.clone(),
        right_transform,
        0xFF0000FF,
    ));

    // Mirror Cube (White/Bright)
    let mirror_transform = Mat4::translation(0.0, 0.0, 2.5) * Mat4::scale(0.5, 0.5, 0.5);
    scene.add_object(SceneObject::new(
        cube_mesh.clone(),
        mirror_transform,
        0xFFFFFFFF,
    ));

    let mut timestep = FixedTimestep::new(60);
    let mut time = 0.0f32;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            time += 0.01;
        }

        // Animate objects
        // Rotate center cube
        let rotation = Mat4::rotation_y(time);
        let translation = Mat4::translation(0.0, 0.5 + (time * 2.0).sin() * 0.5, 0.0);
        // Note: transform order is Scale * Rotate * Translate.
        // We want Translate * Rotate? No, Rotate then Translate relative to parent?
        // Row-vector convention: v * R * T.
        // Rotation applied first, then Translation.
        // This makes object rotate around its local origin, then move to world position.
        scene.objects[1].transform = translation * rotation;
        // Re-calculate AABB logic happens implicitly via transform,
        // but SceneObject stores local_aabb which is static.
        // Raytracer uses calculate_world_aabb which uses transform. Correct.

        renderer.render(&scene, &mut framebuffer);

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
