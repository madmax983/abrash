use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::platform::{Window, WindowBackend};
use abrash::skybox::{Cubemap, draw_skybox};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use std::time::Instant;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn print_banner() {
    println!("\n{}", "🌌 Skybox Demo".bold().cyan());
    println!("{}", "=====================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Renders a cubemap skybox").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Texture"),
            Cell::new("6x Checkerboard faces").fg(Color::Yellow),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");

    println!("\n{}", "🎮 Controls".bold());
    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Mouse"),
            Cell::new("None"),
        ])
        .add_row(vec![
            Cell::new("Keyboard"),
            Cell::new("Auto-rotating camera"),
        ]);
    println!("{controls}\n");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();
    let mut window = Window::new("Abrash - Skybox Demo", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;

    // Create 6 face textures with checkerboard patterns
    // Faces: +X (Right), -X (Left), +Y (Top), -Y (Bottom), +Z (Front), -Z (Back)
    let size = 64;
    let faces = [
        Texture::checkered(size, size, 0xFFFF0000, 0xFFFFFFFF).unwrap(), // Right: Red/White
        Texture::checkered(size, size, 0xFF00FFFF, 0xFFFFFFFF).unwrap(), // Left: Cyan/White
        Texture::checkered(size, size, 0xFF0000FF, 0xFFFFFFFF).unwrap(), // Top: Blue/White
        Texture::checkered(size, size, 0xFFFFFF00, 0xFFFFFFFF).unwrap(), // Bottom: Yellow/White
        Texture::checkered(size, size, 0xFF00FF00, 0xFFFFFFFF).unwrap(), // Front: Green/White
        Texture::checkered(size, size, 0xFFFF00FF, 0xFFFFFFFF).unwrap(), // Back: Magenta/White
    ];
    let cubemap = Cubemap::new(faces);

    // Camera setup
    // FOV 90 degrees (1.57 rad)
    let proj = Mat4::perspective(1.57, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);

    let start_time = Instant::now();

    while window.is_open() {
        window.poll_events();

        let t = start_time.elapsed().as_secs_f32();

        // Rotate view direction
        // Camera at origin, looking around
        let eye = Vec3::new(0.0, 0.0, 0.0);
        let target = Vec3::new(t.sin(), (t * 0.3).cos() * 0.5, t.cos());
        let up = Vec3::new(0.0, 1.0, 0.0);

        let view = Mat4::look_at(eye, target, up);

        framebuffer.clear(0xFF000000);
        zbuffer.clear();

        // Draw Skybox
        draw_skybox(&mut framebuffer, &mut zbuffer, view, proj, &cubemap);

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
