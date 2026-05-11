use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::motion_trails::{MotionTrailsConfig, apply_motion_trails};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Motion Trails Demo".bold().cyan());
    println!("{}", "=====================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Simulates motion trails/ghosting by blending historical frames")
                .fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");

    println!("\n{}", "🎮 Controls".bold());
    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![Cell::new("Mouse"), Cell::new("None")])
        .add_row(vec![
            Cell::new("Keyboard"),
            Cell::new("Close window to exit"),
        ]);
    println!("{controls}\n");
}

fn main() {
    #[cfg(feature = "nova")]
    print_banner();

    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let eye = Vec3::new(0.0, 0.0, 5.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let view = Mat4::look_at(eye, target, up);
    let proj = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 100.0);
    let view_proj = view * proj;

    let mut history = Vec::new();
    fb.clear(0xFF00_0000);
    zb.clear();

    let v0_local = Vec3::new(0.0, 1.0, 0.0);
    let v1_local = Vec3::new(-1.0, -1.0, 0.0);
    let v2_local = Vec3::new(1.0, -1.0, 0.0);

    let v0_clip = view_proj.transform_point(v0_local);
    let v1_clip = view_proj.transform_point(v1_local);
    let v2_clip = view_proj.transform_point(v2_local);

    fill_triangle_3d(&mut fb, &mut zb, v0_clip, v1_clip, v2_clip, 0xFFFF_0000);

    let config = MotionTrailsConfig { decay: 0.8 };

    apply_motion_trails(&mut fb, &mut history, &config);

    // Normally this would be rendered to a window, but this is a minimal example
    // just to show it compiles and runs.
    println!("Motion Trails demo ran successfully!");
}
