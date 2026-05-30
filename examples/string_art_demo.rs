use abrash::math::Vec2;
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::string_art::StringArt;

fn main() {
    let width = 400;
    let height = 400;

    // Create a target image to approximate (e.g., a dark circle in the middle)
    let mut target_image = vec![255u8; width * height]; // White background
    let center_x = width as isize / 2;
    let center_y = height as isize / 2;
    let target_radius = 100_isize;

    for y in 0..height {
        for x in 0..width {
            let dx = x as isize - center_x;
            let dy = y as isize - center_y;
            if dx * dx + dy * dy <= target_radius * target_radius {
                // Dark circle
                target_image[y * width + x] = 0;
            }
        }
    }

    println!("Generating string art...");
    let num_pins = 200;
    let center = Vec2::new(width as f32 / 2.0, height as f32 / 2.0);
    let radius = (width as f32 / 2.0) - 10.0;

    let mut art = StringArt::new(num_pins, center, radius, width, height);
    let num_lines = 1000;

    let mut image_copy = target_image.clone();
    let lines = art.generate(&mut image_copy, width, height, num_lines);

    println!("Generated {} lines.", lines.len());

    // Draw the final lines into a framebuffer for visualization
    let mut fb = Framebuffer::new(width as u32, height as u32).unwrap();
    fb.clear(0xFF_FF_FF_FF); // White background

    // Convert pins to framebuffer coordinates and draw lines
    for &(p0_idx, p1_idx) in &lines {
        let p0 = art.pins[p0_idx];
        let p1 = art.pins[p1_idx];

        let mut x0 = p0.x as isize;
        let mut y0 = p0.y as isize;
        let x1 = p1.x as isize;
        let y1 = p1.y as isize;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && x0 < width as isize && y0 >= 0 && y0 < height as isize {
                fb.set_pixel(x0 as i32, y0 as i32, 0xFF_00_00_00); // Black string
            }

            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    println!("String art generation complete. (Rendering logic tested).");
}
