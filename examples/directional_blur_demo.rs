use abrash::experimental::directional_blur::apply_directional_blur;
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

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn print_banner() {
    println!("\n{}", "✨ Directional Blur Demo".bold().cyan());
    println!("{}", "==========================".dark_grey());

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
            Cell::new("Effect"),
            Cell::new("Directional / Motion Blur").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();
    let mut window = Window::new("Abrash - Directional Blur", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;

    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let eye = Vec3::new(0.0, 2.0, 5.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let view = Mat4::look_at(eye, target, up);
    let camera = Camera::new(view, projection);

    let mut scene = Scene::new(camera);

    let cube_mesh = Arc::new(Mesh::cube(1.0));

    // Floor
    let floor_transform = Mat4::translation(0.0, -1.0, 0.0) * Mat4::scale(10.0, 0.1, 10.0);
    scene.add_object(SceneObject::new(
        cube_mesh.clone(),
        floor_transform,
        0xFF808080,
    ));

    // Moving Cube
    let cube_transform = Mat4::identity();
    scene.add_object(SceneObject::new(cube_mesh, cube_transform, 0xFFFF5555));

    let mut timestep = FixedTimestep::new(60);
    let mut time = 0.0f32;
    let mut last_cube_x = 0.0;
    let mut last_cube_y = 0.0;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            time += 0.016;
        }

        let cube_x = (time * 3.0).sin() * 3.0;
        let cube_y = (time * 5.0).cos().abs() * 2.0;

        let translation = Mat4::translation(cube_x, cube_y, 0.0);
        let rotation = Mat4::rotation_y(time) * Mat4::rotation_z(time * 0.5);
        scene.objects[1].transform = translation * rotation;

        // Render base scene
        framebuffer.clear(0xFF111111);
        let mut zb = abrash::zbuffer::ZBuffer::new(WIDTH, HEIGHT)?;

        let mut renderer = abrash::rasterizer::tile::TileRenderer::new(WIDTH, HEIGHT);
        scene.render(&mut renderer, &mut framebuffer, &mut zb);

        // Compute simulated motion blur vector based on speed
        let dx = cube_x - last_cube_x;
        let dy = cube_y - last_cube_y;

        last_cube_x = cube_x;
        last_cube_y = cube_y;

        // Apply directional blur as a simulated motion blur screen effect
        // Scale the delta drastically to exaggerate the screen-space blur amount
        apply_directional_blur(&mut framebuffer, dx * 500.0, -dy * 500.0, 16);

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
