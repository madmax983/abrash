use abrash::experimental::pixel_sort::apply_pixel_sort;
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::Window;
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF10_1010; // Dark grey

// Face colors for the cube
const COLORS: [u32; 6] = [
    0xFFFF_0000, // Red
    0xFF00_FF00, // Green
    0xFF00_00FF, // Blue
    0xFFFF_FF00, // Yellow
    0xFFFF_00FF, // Magenta
    0xFF00_FFFF, // Cyan
];

fn print_banner() {
    println!("\n{}", "📺 Pixel Sort Demo".bold().cyan());
    println!("{}", "=======================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Glitch art pixel sorting effect").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Features"),
            Cell::new("Luminance Threshold Sorting, Alternating Axes").fg(Color::Yellow),
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
            Cell::new("Auto"),
            Cell::new("Cycles sorting modes over time"),
        ]);
    println!("{controls}\n");
}

fn render_cube(fb: &mut Framebuffer, zb: &mut ZBuffer, cube: &Mesh, model: Mat4, view_proj: Mat4) {
    let mvp = view_proj * model;

    for (face_idx, tri_indices) in cube.indices.iter().enumerate() {
        let v0 = cube.vertices[tri_indices[0]];
        let v1 = cube.vertices[tri_indices[1]];
        let v2 = cube.vertices[tri_indices[2]];

        let (clip0, w0) = mvp.transform_point(v0);
        let (clip1, w1) = mvp.transform_point(v1);
        let (clip2, w2) = mvp.transform_point(v2);

        // Simple backface culling
        if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
            continue;
        }

        let color = COLORS[(face_idx / 2) % 6];
        fill_triangle_3d(fb, zb, (clip0, w0), (clip1, w1), (clip2, w2), color);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();
    let mut window = Window::new("Abrash - Pixel Sort Demo", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;
    let mut timestep = FixedTimestep::new(60);

    let cube = Mesh::cube(1.0);

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 2.0, 5.0),
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let view_proj = projection * view;

    let mut angle_y: f32 = 0.0;
    let mut total_time: f32 = 0.0;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle_y += 1.0 * timestep.dt();
            total_time += timestep.dt();
        }

        framebuffer.clear(BACKGROUND);
        zbuffer.clear();

        // Render Cubes
        let model1 = Mat4::translation(-2.0, 0.0, -2.0) * Mat4::rotation_y(angle_y);
        render_cube(&mut framebuffer, &mut zbuffer, &cube, model1, view_proj);

        let model2 = Mat4::translation(2.0, 0.0, -5.0) * Mat4::rotation_x(angle_y * 0.5);
        render_cube(&mut framebuffer, &mut zbuffer, &cube, model2, view_proj);

        // Modulate pixel sort parameters over time for a dynamic glitch effect
        // Cycle every 4 seconds
        let phase = (total_time % 4.0) / 4.0;

        let threshold = 0.3 + (total_time * 2.0).sin().abs() * 0.4;
        let vertical = phase > 0.5;
        let reverse = (total_time * 0.5).sin() > 0.0;

        // Apply Pixel Sort Effect
        apply_pixel_sort(&mut framebuffer, threshold, vertical, reverse);

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
