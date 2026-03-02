#![cfg(all(feature = "nova", feature = "backend-tui"))]

use abrash::experimental::crepuscular::apply_god_rays;
use abrash::framebuffer::Framebuffer;
use abrash::platform::WindowBackend;
use abrash::platform::tui::TuiWindow;
use abrash::zbuffer::ZBuffer;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::event::{self, Event, KeyCode};
use crossterm::style::Stylize;
use std::time::Duration;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 60;

fn print_banner() {
    println!("\n{}", "☀️  God Rays Demo".bold().cyan());
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
            Cell::new("Volumetric light scattering effect").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Effect"),
            Cell::new("Post-process screen-space god rays").fg(Color::Yellow),
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
            Cell::new("Arrow Keys"),
            Cell::new("Move the light source"),
        ])
        .add_row(vec![Cell::new("Q / Esc"), Cell::new("Quit Demo")]);
    println!("{controls}\n");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();
    let mut window = TuiWindow::new("God Rays Demo", WIDTH, HEIGHT)?;
    let mut fb = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zb = ZBuffer::new(WIDTH, HEIGHT)?;

    let mut light_x = WIDTH as f32 / 2.0;
    let mut light_y = HEIGHT as f32 / 2.0;

    let mut time: f32 = 0.0;

    loop {
        if event::poll(Duration::from_millis(16))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Up => light_y -= 2.0,
                KeyCode::Down => light_y += 2.0,
                KeyCode::Left => light_x -= 2.0,
                KeyCode::Right => light_x += 2.0,
                _ => {}
            }
        }

        fb.clear(0xFF00_0000); // Black background
        zb.clear();

        // 1. Draw "Sun"
        // A simple white rect
        for y in (light_y as i32 - 4)..=(light_y as i32 + 4) {
            for x in (light_x as i32 - 4)..=(light_x as i32 + 4) {
                if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                    fb.set_pixel(x, y, 0xFFFF_FFFF);
                }
            }
        }

        // 2. Draw Occuluders (Moving pillars)
        let pillar_x = (WIDTH as f32 / 2.0 + (time * 2.0).sin() * 20.0) as i32;
        let pillar_w = 10;
        let pillar_y_start = 10;
        let pillar_y_end = HEIGHT as i32 - 10;

        for y in pillar_y_start..pillar_y_end {
            for x in (pillar_x - pillar_w / 2)..=(pillar_x + pillar_w / 2) {
                if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                    // Dark grey occluder
                    fb.set_pixel(x, y, 0xFF22_2222);
                }
            }
        }

        // Another occluder
        let pillar2_x = (WIDTH as f32 / 2.0 + (time * 1.5).cos() * 30.0) as i32;
        for y in (HEIGHT as i32 / 2)..HEIGHT as i32 {
            for x in (pillar2_x - 8)..=(pillar2_x + 8) {
                if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                    fb.set_pixel(x, y, 0xFF11_1111);
                }
            }
        }

        // 3. Apply God Rays
        apply_god_rays(
            &mut fb, light_x, light_y, 1.0,  // density
            0.05, // weight
            0.98, // decay
            1.2,  // exposure
            64,   // samples
        );

        window.blit_framebuffer(&fb);
        time += 0.05;
    }

    Ok(())
}
