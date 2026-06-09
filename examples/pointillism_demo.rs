use abrash::experimental::pointillism::{apply_pointillism, PointillismConfig};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{run_windowed_app, HostError, WindowConfig};
use abrash::texture::Texture;
use comfy_table::Table;

fn run() -> Result<(), HostError> {
    let width = 800;
    let height = 600;

    let mut fb = Framebuffer::new(width, height)?;
    let mut config = PointillismConfig::default();

    println!("Pointillism Filter Demo");
    println!("Controls:");
    println!("  Arrow Keys: Adjust dot spacing");
    println!("  W/S: Adjust dot radius");
    println!("  A/D: Adjust jitter amount");

    // We'll generate a test image: a simple gradient with a circle
    let mut source_fb = Framebuffer::new(width, height)?;
    for y in 0..height {
        for x in 0..width {
            let r = (x as f32 / width as f32 * 255.0) as u32;
            let g = (y as f32 / height as f32 * 255.0) as u32;
            let b = 128;
            let mut color = 0xFF_000000 | (r << 16) | (g << 8) | b;

            // Draw a circle in the center
            let dx = x as f32 - width as f32 / 2.0;
            let dy = y as f32 - height as f32 / 2.0;
            if dx * dx + dy * dy < 150.0 * 150.0 {
                color = 0xFF_FF5555;
            }
            source_fb.set_pixel(x as u32, y as u32, color);
        }
    }

    run_windowed_app(
        WindowConfig {
            width: width as u32,
            height: height as u32,
            title: "Pointillism Demo".to_string(),
            ..Default::default()
        },
        move |ctx| {
            // Apply controls
            if ctx.keys.pressed("ArrowUp") {
                config.dot_spacing += 0.5;
            }
            if ctx.keys.pressed("ArrowDown") {
                config.dot_spacing = (config.dot_spacing - 0.5).max(1.0);
            }
            if ctx.keys.pressed("w") {
                config.dot_radius += 0.5;
            }
            if ctx.keys.pressed("s") {
                config.dot_radius = (config.dot_radius - 0.5).max(1.0);
            }
            if ctx.keys.pressed("d") {
                config.jitter_amount += 0.5;
            }
            if ctx.keys.pressed("a") {
                config.jitter_amount = (config.jitter_amount - 0.5).max(0.0);
            }

            // Copy source image to fb
            fb.pixels_mut().copy_from_slice(source_fb.pixels());

            // Apply effect
            apply_pointillism(&mut fb, &config);

            ctx.framebuffer.pixels_mut().copy_from_slice(fb.pixels());
            Ok(())
        },
    )?;

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Initialization Error:");
        let mut table = Table::new();
        table.add_row(vec![e.to_string()]);
        println!("{table}");
    }
}
