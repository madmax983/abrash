use abrash::experimental::kaleidoscope::apply_kaleidoscope;
use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::draw_line_3d;
use abrash::zbuffer::ZBuffer;
use std::time::Instant;

fn main() {
    let width = 400;
    let height = 400;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let start_time = Instant::now();

    for _ in 0..10 {
        fb.clear(0xFF000000); // Black
        zb.clear();

        // Draw a triangle
        draw_line_3d(
            &mut fb,
            &mut zb,
            (Vec3::new(width as f32 / 2.0, height as f32 / 2.0, 0.0), 1.0),
            (Vec3::new(width as f32, 0.0, 0.0), 1.0),
            0xFFFF0000,
        );
        draw_line_3d(
            &mut fb,
            &mut zb,
            (Vec3::new(width as f32, 0.0, 0.0), 1.0),
            (Vec3::new(width as f32, height as f32 / 2.0, 0.0), 1.0),
            0xFF00FF00,
        );
        draw_line_3d(
            &mut fb,
            &mut zb,
            (Vec3::new(width as f32, height as f32 / 2.0, 0.0), 1.0),
            (Vec3::new(width as f32 / 2.0, height as f32 / 2.0, 0.0), 1.0),
            0xFF0000FF,
        );

        // Draw a circle
        let center_x = width as f32 * 0.75;
        let center_y = height as f32 * 0.25;
        let radius = 50.0;

        for i in 0..360 {
            let rad = (i as f32).to_radians();
            let x = (center_x + rad.cos() * radius) as i32;
            let y = (center_y + rad.sin() * radius) as i32;
            fb.set_pixel(x, y, 0xFFFFFF00);
        }

        // Apply kaleidoscope
        apply_kaleidoscope(&mut fb, 6, 0.5);
    }

    let duration = start_time.elapsed();
    println!("Kaleidoscope effect benchmark: {:?}", duration);
    println!("Note: Headless benchmark, run visually with TUI for actual display.");
}
