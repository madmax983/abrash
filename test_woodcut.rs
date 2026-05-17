use std::f32::consts::PI;

fn main() {
    let width = 80;
    let height = 40;

    // Create a dummy "image" (a radial gradient / sphere)
    let mut image = vec![0.0f32; width * height];
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let radius = 20.0;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let dist = (dx * dx + dy * dy * 4.0).sqrt(); // scale Y for aspect ratio
            let luminance = if dist < radius {
                dist / radius // 0 at center (dark), 1 at edge (bright)
            } else {
                1.0 // white background
            };
            image[y * width + x] = luminance;
        }
    }

    // Multi-directional Woodcut / Engraving (cross-hatch depending on darkness)
    let base_freq = 0.5;

    for y in 0..height {
        for x in 0..width {
            let lum = image[y * width + x];

            let angle1 = PI / 4.0; // 45 deg
            let cos1 = angle1.cos();
            let sin1 = angle1.sin();
            let v1 = -(x as f32) * sin1 + y as f32 * cos1;
            let wave1 = (v1 * base_freq).sin();

            let angle2 = -PI / 4.0; // -45 deg
            let cos2 = angle2.cos();
            let sin2 = angle2.sin();
            let v2 = -(x as f32) * sin2 + y as f32 * cos2;
            let wave2 = (v2 * base_freq).sin();

            let angle3: f32 = 0.0; // Horizontal
            let cos3 = angle3.cos();
            let sin3 = angle3.sin();
            let v3 = -(x as f32) * sin3 + y as f32 * cos3;
            let wave3 = (v3 * base_freq).sin();

            // The darker it is, the more layers of hatching we apply.
            // Layer 1 threshold
            let t1 = (lum * 1.5) - 0.5;
            let mut ink = wave1 > t1;

            if lum < 0.6 {
                let t2 = (lum * 2.0) - 0.5;
                ink = ink || wave2 > t2;
            }

            if lum < 0.3 {
                let t3 = (lum * 3.0) - 0.5;
                ink = ink || wave3 > t3;
            }

            if ink {
                print!("#");
            } else {
                print!(" ");
            }
        }
        println!();
    }
}
