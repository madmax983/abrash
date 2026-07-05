use abrash::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
use abrash_render::experimental::braille::render_braille;
use std::f32::consts::PI;

fn main() {
    let width = 80;
    let height = 40;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Draw a circle
    let center = Vec2::new(width as f32 / 2.0, height as f32 / 2.0);
    let radius = 15.0;

    for y in 0..height {
        for x in 0..width {
            let p = Vec2::new(x as f32, y as f32);
            let dist = (p.x - center.x).hypot(p.y - center.y);

            // Antialiased edge
            if dist < radius {
                fb.set_pixel(x as i32, y as i32, 0xFFFF_FFFF);
            } else if dist < radius + 1.0 {
                let alpha = (1.0 - (dist - radius)) * 255.0;
                let c = alpha as u32;
                let color = 0xFF00_0000 | (c << 16) | (c << 8) | c;
                fb.set_pixel(x as i32, y as i32, color);
            }
        }
    }

    // Draw a sine wave
    for x in 0..width {
        let normalized_x = x as f32 / width as f32;
        let y = (center.y + (normalized_x * PI * 4.0).sin() * 10.0) as i32;
        if y >= 0 && (y as u32) < height {
            fb.set_pixel(x as i32, y, 0xFFFF_FFFF);
        }
    }

    let braille_output = render_braille(&fb, 128);
    println!("{braille_output}");
}
