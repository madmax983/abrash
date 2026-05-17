use std::f32::consts::PI;

fn main() {
    let width = 80;
    let height = 40;

    // Create a dummy image (e.g., horizontal stripes representing ground and sky)
    let mut image = vec![0.0f32; width * height];
    for y in 0..height {
        for x in 0..width {
            image[y * width + x] = if y > height / 2 { 0.2 } else { 0.8 }; // Ground vs sky
        }
    }

    // Tiny Planet (Stereographic projection mapping)
    // Actually, simple polar to cartesian mapping for a panoramic image works:
    // (x, y) on screen mapped to (angle, radius) on source image

    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let max_radius = height as f32 / 2.0;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;

            let radius = (dx * dx + dy * dy * 4.0).sqrt(); // scale Y for terminal aspect ratio
            let mut angle = dy.atan2(dx);
            if angle < 0.0 {
                angle += 2.0 * PI;
            }

            // Map angle [0, 2PI] to src_x [0, width]
            let src_x = (angle / (2.0 * PI) * width as f32) as usize;
            // Map radius [0, max_radius] to src_y [height, 0] (ground at center, sky at edge)
            let mut src_y = height as f32 - (radius / max_radius * height as f32);

            if src_y < 0.0 {
                src_y = 0.0;
            } else if src_y >= height as f32 {
                src_y = height as f32 - 1.0;
            }

            let src_x = src_x.clamp(0, width - 1);
            let src_y = src_y as usize;

            let lum = image[src_y * width + src_x];
            if radius > max_radius {
                print!(" "); // outside planet
            } else if lum < 0.5 {
                print!("#"); // ground
            } else {
                print!("."); // sky
            }
        }
        println!();
    }
}
