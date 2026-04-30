#[cfg(feature = "nova")]
use abrash::experimental::chroma_key::smooth_chroma_key;
use abrash::framebuffer::Framebuffer;
use abrash::texture::Texture;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn create_checkerboard_texture(width: u32, height: u32) -> Texture {
    let mut tex = Texture::new(width, height).unwrap();
    for y in 0..height {
        for x in 0..width {
            let color = if ((x / 16) + (y / 16)) % 2 == 0 {
                0xFF_888888
            } else {
                0xFF_444444
            };
            tex.set_pixel(x, y, color);
        }
    }
    tex
}

fn create_green_screen_fg(width: u32, height: u32) -> Framebuffer {
    let mut fb = Framebuffer::new(width, height).unwrap();
    // Green screen background
    fb.clear(0xFF_00FF00);

    // Draw a red square in the center
    let center_x = width as usize / 2;
    let center_y = height as usize / 2;
    let size = 100;

    for y in center_y - size..center_y + size {
        for x in center_x - size..center_x + size {
            // Add a little gradient to show smooth chroma key
            let dr = x as i32 - center_x as i32;
            let dg = y as i32 - center_y as i32;
            let dist = ((dr * dr + dg * dg) as f32).sqrt();

            if dist < size as f32 {
                fb.set_pixel(x as i32, y as i32, 0xFF_FF0000);
            } else if dist < size as f32 + 20.0 {
                // Blend from red to green
                let alpha = (size as f32 + 20.0 - dist) / 20.0;
                let inv_alpha = 1.0 - alpha;
                let r = (255.0 * alpha + 0.0 * inv_alpha) as u32;
                let g = (0.0 * alpha + 255.0 * inv_alpha) as u32;
                fb.set_pixel(x as i32, y as i32, 0xFF_000000 | (r << 16) | (g << 8));
            }
        }
    }
    fb
}

fn print_banner() {
    println!("\n{}", "🌟 Chroma Key Demo".bold().cyan());
    println!("{}", "==================".dark_grey());

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
            Cell::new("Smooth Chroma Key compositing").fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Renderer"),
            Cell::new("Software Post-Process").fg(Color::Yellow),
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
        .add_row(vec![Cell::new("Keyboard"), Cell::new("None")]);
    println!("{controls}\n");
}

fn main() {
    print_banner();

    let width = 640;
    let height = 480;

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("🌟 Nova: Chroma Key Demo")
        .with_inner_size(winit::dpi::LogicalSize::new(width, height))
        .build(&event_loop)
        .unwrap();

    let window = Arc::new(window);
    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

    let mut bg = Framebuffer::new(width, height).unwrap();
    let tex = create_checkerboard_texture(256, 256);

    // Fill bg with checkerboard
    for y in 0..height {
        for x in 0..width {
            let u = (x as f32 / width as f32) * 4.0;
            let v = (y as f32 / height as f32) * 4.0;
            let color = tex.get_pixel_bilinear(u, v);
            bg.set_pixel(x as i32, y as i32, color);
        }
    }

    #[allow(unused_mut)]
    let mut fg = create_green_screen_fg(width, height);

    // Apply smooth chroma key
    // Key color: Pure green (0xFF_00FF00)
    // Threshold: Match exactly green (0.0) or close to it
    // Feather: 100.0 for smooth blending
    #[cfg(feature = "nova")]
    smooth_chroma_key(&mut fg, &bg, 0xFF_00FF00, 20.0, 100.0);

    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => {
                elwt.exit();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                surface
                    .resize(
                        NonZeroU32::new(width).unwrap(),
                        NonZeroU32::new(height).unwrap(),
                    )
                    .unwrap();

                let mut buffer = surface.buffer_mut().unwrap();
                for (i, pixel) in fg.as_slice().iter().enumerate() {
                    buffer[i] = *pixel;
                }
                buffer.present().unwrap();
            }
            _ => {}
        }
    });
}
