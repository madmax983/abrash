use abrash::framebuffer::Framebuffer;
use abrash::platform::Window;
use abrash::primitives::draw_polygon;
use abrash::shapes::Polygon;
use abrash::math::{Vec2, Mat2};
use abrash::time::FixedTimestep;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF000000;
const FOREGROUND: u32 = 0xFF00FF00;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Abrash - Rotating Polygon (Optimized)", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let mut timestep = FixedTimestep::new(60);

    // Create base octagon (will be cloned and transformed each frame)
    let base_polygon = Polygon::regular(8, 150.0);
    let center = Vec2::new(WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0);

    let mut angle: f32 = 0.0;
    let rotation_speed: f32 = 1.0;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle += rotation_speed * timestep.dt();
        }

        framebuffer.clear(BACKGROUND);

        // Use in-place transforms (avoids extra allocations)
        let rotation = Mat2::rotation(angle);
        let mut transformed = base_polygon.clone();
        transformed.transform_in_place(&rotation);
        transformed.translate_in_place(center);

        draw_polygon(&mut framebuffer, &transformed, FOREGROUND);

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
