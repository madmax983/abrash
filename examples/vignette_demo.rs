//! Demonstration of the Nova Vignette filter.

use abrash::experimental::vignette::{VignetteConfig, apply_vignette};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{Event, Window};

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "📷 Vignette Filter Demo".bold().cyan());
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
            Cell::new("Cinematic edge-darkening effect").fg(Color::Green),
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
        .add_row(vec![Cell::new("Keyboard"), Cell::new("Auto-pulsating")]);
    println!("{controls}\n");
}

fn main() {
    print_banner();
    let mut window = Window::new("Vignette Filter Demo (Nova)", 800, 600).unwrap();

    let width = 800;
    let height = 600;

    let mut fb = Framebuffer::new(width, height).unwrap();

    let mut angle: f32 = 0.0;

    // Draw a colorful grid pattern onto a persistent buffer
    let mut original_fb = Framebuffer::new(width, height).unwrap();
    for y in 0..height {
        for x in 0..width {
            let cx = x / 50;
            let cy = y / 50;
            let is_white = (cx + cy) % 2 == 0;
            let color = if is_white {
                0xFF_DD_EE_FF
            } else {
                0xFF_22_44_88
            };
            original_fb.set_pixel(x as i32, y as i32, color);
            // Draw a center element
            if (x - 400) * (x - 400) + (y - 300) * (y - 300) < 10000 {
                original_fb.set_pixel(x as i32, y as i32, 0xFF_FF_AA_00);
            }
        }
    }

    while window.is_open() {
        let events = window.poll_events();
        for event in events {
            if matches!(event, Event::Close) {
                return;
            }
        }

        angle += 0.05;

        // Reset framebuffer manually using slice copy
        fb.as_mut_slice().copy_from_slice(original_fb.as_slice());

        // Pulsate the intensity over time to make the demo dynamic
        let intensity = 0.7 + (angle * 0.5).sin() * 0.3; // Ranges from 0.4 to 1.0

        // Oscillate outer radius
        let outer_radius = 1.0 + (angle * 0.3).cos() * 0.2; // 0.8 to 1.2

        let config = VignetteConfig {
            center_x: 0.5,
            center_y: 0.5,
            inner_radius: 0.1,
            outer_radius,
            intensity,
        };

        // Apply the vignette filter
        apply_vignette(&mut fb, &config);

        window.blit_framebuffer(&fb);
    }
}
