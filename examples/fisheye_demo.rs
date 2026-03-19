//! Demonstration of the Nova Fisheye Lens Distortion filter.

use abrash::experimental::fisheye::{FisheyeConfig, apply_fisheye};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{Event, Window};

use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

fn print_banner() {
    println!("\n{}", "📷 Fisheye Filter Demo".bold().cyan());
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
            Cell::new("Ultra-wide lens bulging distortion").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Renderer"),
            Cell::new("Software Rasterizer + Post-Process").fg(Color::Yellow),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
    println!("\n{}", "⌨️  Controls".bold());
    println!("  • [ESC] to quit");
    println!("  • Animated automatically\n");
}

fn main() {
    print_banner();
    let mut window = Window::new("Fisheye Filter Demo (Nova)", 800, 600).unwrap();

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

        // Animate strength
        angle += 0.02;
        let animated_strength = angle.sin() * 0.5 + 0.3; // Oscillates between -0.2 and 0.8

        let config = FisheyeConfig {
            strength: animated_strength,
            zoom: 1.1, // Slight zoom to reduce black borders
        };

        // Copy the original unharmed image into the working buffer
        fb.as_mut_slice().copy_from_slice(original_fb.as_slice());

        // Apply the post processing filter
        apply_fisheye(&mut fb, &config);

        window.blit_framebuffer(&fb);
    }
}
