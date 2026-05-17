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

    // Apply Engraving
    // Make frequency depend on luminosity to make it bend/warp around the shape (banknote style)
    let base_freq = 0.5;
    let angle = PI / 6.0; // 30 degrees
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    for y in 0..height {
        for x in 0..width {
            let lum = image[y * width + x];

            // Rotate coordinates for the wave
            let _u = x as f32 * cos_a + y as f32 * sin_a;
            let v = -(x as f32) * sin_a + y as f32 * cos_a;

            // Add a sine perturbation based on u to make the lines squiggly based on luminance
            let perturbation = (1.0 - lum) * (_u * 0.2).sin() * 2.0;

            // Generate wave
            let wave = ((v + perturbation) * base_freq).sin();

            // mapped threshold: if lum = 1.0 (white), threshold = 1.0 (no ink)
            // if lum = 0.0 (black), threshold = -1.0 (full ink)
            let threshold = (lum * 2.0) - 1.0;

            if wave > threshold {
                print!("#");
            } else {
                print!(" ");
            }
        }
        println!();
    }
}
