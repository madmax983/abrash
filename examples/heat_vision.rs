use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::post_process::apply_heat_vision;
use abrash::mesh::Mesh;
use abrash::platform::{Window, WindowBackend};
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
// Use black background to make heat vision pop
const BACKGROUND: u32 = 0xFF00_0000;

fn print_banner() {
    println!("\n{}", "🔥 Heat Vision Demo".bold().red());
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
            Cell::new("Simulates thermal imaging effect").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Effect"),
            Cell::new("Post-process heat map shader").fg(Color::Yellow),
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
    let mut window = Window::new("Abrash - Heat Vision Demo", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;
    let mut timestep = FixedTimestep::new(60);

    // Create a scene with multiple objects at different depths to show off the effect
    let cube = Mesh::cube(1.0);

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 3.0, 6.0), // High up
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    let mut angle: f32 = 0.0;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle += 0.5 * timestep.dt();
        }

        framebuffer.clear(BACKGROUND);
        zbuffer.clear();

        // Render 3 cubes at different depths

        // Cube 1: Close (Left)
        let model1 = Mat4::rotation_y(angle) * Mat4::translation(-2.0, 0.0, 1.0);

        // Cube 2: Center (Mid)
        let model2 = Mat4::rotation_x(angle * 0.5) * Mat4::rotation_z(angle * 0.3);

        // Cube 3: Far (Right, Back)
        let model3 = Mat4::rotation_y(-angle * 0.5) * Mat4::translation(2.0, 0.0, -2.0);

        let models = [model1, model2, model3];

        for model in models {
            let mvp = projection * (view * model);

            for tri_indices in &cube.indices {
                let v0 = cube.vertices[tri_indices[0]];
                let v1 = cube.vertices[tri_indices[1]];
                let v2 = cube.vertices[tri_indices[2]];

                let (clip0, w0) = mvp.transform_point(v0);
                let (clip1, w1) = mvp.transform_point(v1);
                let (clip2, w2) = mvp.transform_point(v2);

                if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                    continue;
                }

                // Color doesn't matter for heat vision, but we render white to populate Z-buffer
                fill_triangle_3d(
                    &mut framebuffer,
                    &mut zbuffer,
                    (clip0, w0),
                    (clip1, w1),
                    (clip2, w2),
                    0xFFFFFFFF,
                );
            }
        }

        // Apply the Heat Vision effect!
        apply_heat_vision(&mut framebuffer, &zbuffer);

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
