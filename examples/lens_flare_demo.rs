use abrash::experimental::lens_flare::{LensFlareConfig, apply_lens_flare};
use abrash::framebuffer::Framebuffer;
use abrash::math::Vec2;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Clear background to a dark color
    fb.clear(0xFF_050510);

    // Draw a fake "sun" at the light position
    let light_pos = Vec2::new(600.0, 150.0);

    // Simple circle for sun
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let dx = x as f32 - light_pos.x;
            let dy = y as f32 - light_pos.y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < 900.0 {
                fb.set_pixel(x, y, 0xFF_FFFFFF); // White sun
            } else if dist_sq < 2500.0 {
                // simple falloff
                let dist = dist_sq.sqrt();
                let intensity = 255.0 * (1.0 - (dist - 30.0) / 20.0);
                let intensity = intensity.max(0.0).min(255.0) as u32;
                let bg_a = 0xFF;
                let bg_r = 0x05;
                let bg_g = 0x05;
                let bg_b = 0x10;

                let r = (bg_r + intensity).min(255);
                let g = (bg_g + intensity).min(255);
                let b = (bg_b + intensity).min(255);
                let color = (bg_a << 24) | (r << 16) | (g << 8) | b;
                fb.set_pixel(x, y, color);
            }
        }
    }

    let config = LensFlareConfig::default();
    apply_lens_flare(&mut fb, light_pos, &config);

    // Save to PPM
    let path = Path::new("lens_flare.ppm");
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    use std::io::Write;
    writeln!(writer, "P3")?;
    writeln!(writer, "{} {}", width, height)?;
    writeln!(writer, "255")?;

    for pixel in fb.as_slice() {
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;
        writeln!(writer, "{} {} {}", r, g, b)?;
    }

    println!("Saved lens_flare.ppm");
    Ok(())
}
