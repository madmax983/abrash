use abrash::experimental::anaglyph::{apply_anaglyph, AnaglyphConfig};
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::Window;
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

use comfy_table::{presets, Cell, Color, Table};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF00_0000;

// Face colors for the cube
const COLORS: [u32; 6] = [
    0xFFFF_0000, // Red - front
    0xFF00_FF00, // Green - back
    0xFF00_00FF, // Blue - top
    0xFFFF_FF00, // Yellow - bottom
    0xFFFF_00FF, // Magenta - right
    0xFF00_FFFF, // Cyan - left
];

fn print_banner() {
    println!("\n{}", "👓 Anaglyph 3D Demo".bold().cyan());
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
            Cell::new("Stereoscopic 3D rendering").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Renderer"),
            Cell::new("Software Rasterizer + Post-Process").fg(Color::Yellow),
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
        .add_row(vec![Cell::new("Mouse"), Cell::new("None")])
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-rotating")]);
    println!("{controls}\n");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();
    let mut window = Window::new("Abrash - Anaglyph 3D", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;
    let mut timestep = FixedTimestep::new(60);

    let cube = Mesh::cube(1.0);

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 1.5, 3.0), // eye
        Vec3::new(0.0, 0.0, 0.0), // target
        Vec3::new(0.0, 1.0, 0.0), // up
    );

    let mut angle_y: f32 = 0.0;
    let mut angle_x: f32 = 0.0;

    let anaglyph_config = AnaglyphConfig {
        max_offset: 20, // Strong enough to be visible
        focal_depth: 3.5, // Depth where image converges (roughly around the cube)
    };

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle_y += 1.0 * timestep.dt();
            angle_x += 0.5 * timestep.dt();
        }

        framebuffer.clear(BACKGROUND);
        zbuffer.clear();

        // Model matrix (rotation)
        let model = Mat4::rotation_y(angle_y) * Mat4::rotation_x(angle_x);

        // MVP matrix
        let mvp = projection * (view * model);

        // Transform and render each triangle
        for (face_idx, tri_indices) in cube.indices.iter().enumerate() {
            let v0 = cube.vertices[tri_indices[0]];
            let v1 = cube.vertices[tri_indices[1]];
            let v2 = cube.vertices[tri_indices[2]];

            // Transform vertices
            let (clip0, w0) = mvp.transform_point(v0);
            let (clip1, w1) = mvp.transform_point(v1);
            let (clip2, w2) = mvp.transform_point(v2);

            // Simple backface culling (check if facing camera)
            // Skip if all w values are negative (behind camera)
            if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                continue;
            }

            // We use grayscale colors to make the stereoscopic effect cleaner
            // Standard colors can cause issues in anaglyph if they lack red or blue/green
            let intensity = (face_idx as u32 * 30 + 100).min(255);
            let color = 0xFF00_0000 | (intensity << 16) | (intensity << 8) | intensity;

            fill_triangle_3d(
                &mut framebuffer,
                &mut zbuffer,
                (clip0, w0),
                (clip1, w1),
                (clip2, w2),
                color,
            );
        }

        // Add some more depth layers
        // Let's add a floor
        let floor_model = Mat4::translation(0.0, -1.0, 0.0) * Mat4::scale(5.0, 0.1, 5.0);
        let floor_mvp = projection * (view * floor_model);

        for (face_idx, tri_indices) in cube.indices.iter().enumerate() {
            let v0 = cube.vertices[tri_indices[0]];
            let v1 = cube.vertices[tri_indices[1]];
            let v2 = cube.vertices[tri_indices[2]];

            let (clip0, w0) = floor_mvp.transform_point(v0);
            let (clip1, w1) = floor_mvp.transform_point(v1);
            let (clip2, w2) = floor_mvp.transform_point(v2);

            if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 { continue; }

            // Dark grey floor
            fill_triangle_3d(
                &mut framebuffer,
                &mut zbuffer,
                (clip0, w0),
                (clip1, w1),
                (clip2, w2),
                0xFF_444444,
            );
        }

        // Apply Anaglyph 3D post-processing
        apply_anaglyph(&mut framebuffer, &zbuffer, anaglyph_config);

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
