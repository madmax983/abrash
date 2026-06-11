use abrash::experimental::string_art::apply_string_art;
use abrash::framebuffer::Framebuffer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Loading reference texture...");
    // Let's create a procedural gradient circle as our input to string art
    let width = 800;
    let height = 800;
    let mut fb = Framebuffer::new(width, height)?;

    // Fill with a radial gradient to test string art
    fb.clear(0xFFFF_FFFF); // White background
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let radius = 350.0;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = dx.hypot(dy);
            if dist < radius {
                // A neat pattern: rings based on sine
                let intensity = (dist / 10.0).sin().abs() * (1.0 - dist / radius);
                let gray = ((1.0 - intensity) * 255.0) as u32;
                unsafe {
                    fb.set_pixel_unchecked(
                        x as usize,
                        y as usize,
                        0xFF00_0000 | (gray << 16) | (gray << 8) | gray,
                    );
                }
            }
        }
    }

    // Save the input
    fb.export_ppm("string_art_input.ppm")?;
    println!("Saved input pattern to string_art_input.ppm");

    println!("Generating string art...");
    let num_pins = 256;
    let num_lines = 4000;
    let line_opacity = 0.2;
    let string_color = 0xFF00_0000; // Black strings
    let bg_color = 0xFFFF_FFFF; // White canvas

    apply_string_art(
        &mut fb,
        num_pins,
        num_lines,
        line_opacity,
        string_color,
        bg_color,
    );

    fb.export_ppm("string_art_output.ppm")?;
    println!("Saved result to string_art_output.ppm");

    Ok(())
}
