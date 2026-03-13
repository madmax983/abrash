use abrash::experimental::vision::{VisionConfig, VisionMode, apply_vision};
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::Window;
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;
use std::io::{Write, stdout};

use comfy_table::{Cell, Color, Table, presets};
use crossterm::{cursor, execute, style::Stylize, terminal::{Clear, ClearType}};

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
    println!("\n{}", "👁️  Vision Demo".bold().cyan());
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
            Cell::new("Post-processing effects demo").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Modes"),
            Cell::new("Night, Thermal, Sonar").fg(Color::Yellow),
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
        .add_row(vec![Cell::new("Auto"), Cell::new("Cycles modes every 3s")]);
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
    let mut window = Window::new("Abrash - Vision Demo", WIDTH, HEIGHT)?;
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

    let mut vision_config = VisionConfig {
        mode: VisionMode::Night,
        time: 0.0,
        intensity: 1.0,
    };

    let mut frame_count = 0;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle_y += 1.0 * timestep.dt();
            vision_config.time += timestep.dt();
        }

        // Cycle modes every 180 frames (3 seconds at 60fps)
        frame_count += 1;
        if frame_count % 180 == 0 {
            vision_config.mode = match vision_config.mode {
                VisionMode::Night => VisionMode::Thermal,
                VisionMode::Thermal => VisionMode::Sonar,
                VisionMode::Sonar => VisionMode::Night,
            };
            // Print to console to inform user cleanly
            let mut out = stdout();
            let _ = execute!(
                out,
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine)
            );
            print!("🔄 Mode Switched to: {}", format!("{:?}", vision_config.mode).bold().green());
            let _ = out.flush();
        }

        framebuffer.clear(BACKGROUND);
        zbuffer.clear();

        // Render 3 cubes at different depths

        // Cube 1: Close (-2.0)
        let model1 = Mat4::translation(-2.0, 0.0, -2.0) * Mat4::rotation_y(angle_y);
        render_cube(&mut framebuffer, &mut zbuffer, &cube, model1, view_proj);

        // Cube 2: Medium (-7.0)
        let model2 = Mat4::translation(0.0, 0.0, -7.0) * Mat4::rotation_x(angle_y * 0.5);
        render_cube(&mut framebuffer, &mut zbuffer, &cube, model2, view_proj);

        // Cube 3: Far (-12.0)
        let model3 = Mat4::translation(2.0, 0.0, -12.0) * Mat4::rotation_z(angle_y * 0.3);
        render_cube(&mut framebuffer, &mut zbuffer, &cube, model3, view_proj);

        // Apply Vision Effect
        apply_vision(&mut framebuffer, &zbuffer, &vision_config);

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
