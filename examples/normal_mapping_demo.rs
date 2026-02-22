//! Abrash Graphics Demo - Normal Mapping
//!
//! Demonstrates normal mapping optimization.

use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec2, Vec3, Vec4};
use abrash::platform::{Window, WindowBackend};
use abrash::rasterizer::fill_triangle_normal_mapped;
use abrash::texture::Texture;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn print_banner() {
    println!("\n{}", "🧱 Normal Mapping Demo".bold().blue());
    println!("{}", "======================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Per-pixel lighting with normal maps").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Technique"),
            Cell::new("Tangent space calculation").fg(Color::Magenta),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");

    println!("\n{}", "🎮 Controls".bold());
    println!(" • Mouse: None");
    println!(" • Keyboard: Auto-rotating light source\n");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();
    let mut window = Window::new("Abrash - Normal Mapping", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT).unwrap();

    // Create textures
    // Diffuse: Grey
    let diffuse_map = Texture::checkered(256, 256, 0xFF808080, 0xFF808080).unwrap();

    // Normal Map: Create a "bump" in the center
    // Flat normal is (0.5, 0.5, 1.0) -> 0x8080FF
    let mut normal_map = Texture::new(256, 256).unwrap();
    let normal_map_pixels = normal_map.pixels_mut();

    // Initialize default flat
    normal_map_pixels.fill(0xFFFF8080);

    for y in 0..256 {
        for x in 0..256 {
            let dx = (x as f32 - 128.0) / 128.0;
            let dy = (y as f32 - 128.0) / 128.0;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < 0.8 {
                // Sphere/Hemisphere normal
                // z = sqrt(1 - x^2 - y^2)
                let z = (1.0 - dist * dist).sqrt();
                let nx = dx; // Simplified
                let ny = dy;
                let nz = z;

                let n = Vec3::new(nx, ny, nz).normalize();

                let r = ((n.x * 0.5 + 0.5) * 255.0) as u32;
                let g = ((n.y * 0.5 + 0.5) * 255.0) as u32;
                let b = ((n.z * 0.5 + 0.5) * 255.0) as u32;

                normal_map_pixels[y * 256 + x] = 0xFF000000 | (r << 16) | (g << 8) | b;
            } else {
                normal_map_pixels[y * 256 + x] = 0xFF8080FF;
            }
        }
    }

    // Quad vertices
    // Position, UV, Normal, Tangent
    let p0 = Vec3::new(-2.0, 2.0, 0.0);
    let p1 = Vec3::new(-2.0, -2.0, 0.0);
    let p2 = Vec3::new(2.0, -2.0, 0.0);
    let p3 = Vec3::new(2.0, 2.0, 0.0);

    let uv0 = Vec2::new(0.0, 0.0);
    let uv1 = Vec2::new(0.0, 1.0);
    let uv2 = Vec2::new(1.0, 1.0);
    let uv3 = Vec2::new(1.0, 0.0);

    let n = Vec3::new(0.0, 0.0, 1.0);
    let t = Vec4::new(1.0, 0.0, 0.0, 1.0);

    // Camera
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 4.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    let mut timestep = FixedTimestep::new(60);
    let mut light_angle = 0.0f32;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            light_angle += 0.05;
        }

        framebuffer.clear(0xFF000000);
        zbuffer.clear();

        // Rotating light
        let light_dir = Vec3::new(light_angle.cos(), light_angle.sin() * 0.5, -1.0).normalize();
        let light_color = Vec3::new(1.0, 1.0, 1.0);
        let ambient = Vec3::new(0.1, 0.1, 0.1);

        let mvp = projection * view; // Model is identity

        // Transform vertices to Clip Space
        let v0 = mvp.transform_point(p0);
        let v1 = mvp.transform_point(p1);
        let v2 = mvp.transform_point(p2);
        let v3 = mvp.transform_point(p3);

        // Normals/Tangents in World Space (Identity model matrix)
        let nw = n;
        let tw = t;

        // Render Quad (2 tris)
        // Tri 1: 0, 1, 2
        fill_triangle_normal_mapped(
            &mut framebuffer,
            &mut zbuffer,
            (v0, uv0, nw, tw),
            (v1, uv1, nw, tw),
            (v2, uv2, nw, tw),
            &diffuse_map,
            &normal_map,
            light_dir,
            light_color,
            ambient,
        );

        // Tri 2: 0, 2, 3
        fill_triangle_normal_mapped(
            &mut framebuffer,
            &mut zbuffer,
            (v0, uv0, nw, tw),
            (v2, uv2, nw, tw),
            (v3, uv3, nw, tw),
            &diffuse_map,
            &normal_map,
            light_dir,
            light_color,
            ambient,
        );

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
