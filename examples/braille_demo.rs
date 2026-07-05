use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
use abrash_render::experimental::braille::BrailleConverter;

fn main() {
    let width = 64;
    let height = 32;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Draw a circle
    let center = Vec2::new((width / 2) as f32, (height / 2) as f32);
    let radius = 12.0;

    for y in 0..height {
        for x in 0..width {
            let p = Vec2::new(x as f32, y as f32);
            let dist = (p.x - center.x).hypot(p.y - center.y);
            if (dist - radius).abs() < 1.0 {
                fb.set_pixel(x as i32, y as i32, 0xFFFF_FFFF);
            }
        }
    }

    let converter = BrailleConverter::new(&fb, 128);
    println!("{}", converter.to_string());
}
