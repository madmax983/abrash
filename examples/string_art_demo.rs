use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
use abrash_render::experimental::string_art::StringArt;
use std::time::Instant;

fn main() {
    println!("🌟 Nova: String Art Demo!");

    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    let points = 200;
    let radius = 250.0;
    let center = Vec2::new(width as f32 / 2.0, height as f32 / 2.0);

    // Render a Cardioid
    let multiplier = 2.0;
    let art = StringArt::new(points, multiplier, radius, center);

    let start = Instant::now();
    art.render(&mut fb);
    let duration = start.elapsed();

    println!("Rendered string art with {} points and multiplier {} in {:?}", points, multiplier, duration);

    // Check pixels
    let mut drawn = 0;
    for &pixel in fb.as_slice() {
        if pixel != 0xFF00_0000 {
            drawn += 1;
        }
    }
    println!("Drawn {} pixels.", drawn);

    // In a real demo this would write to a PNG or use the TUI/Windowing backend.
    println!("Done.");
}
