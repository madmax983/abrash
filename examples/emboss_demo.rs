use abrash::experimental::emboss::apply_emboss;
use abrash::framebuffer::Framebuffer;

fn main() {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Draw some simple shapes
    for y in 0..height {
        for x in 0..width {
            let color = if (x / 50 + y / 50) % 2 == 0 {
                0xFFFFFFFF // White
            } else {
                0xFF808080 // Gray
            };
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    // Apply the emboss filter
    apply_emboss(&mut fb);

    println!("Emboss filter applied successfully to a {width}x{height} framebuffer.");
}
