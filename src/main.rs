use abrash::framebuffer::Framebuffer;
use abrash::platform::Window;
use abrash::primitives::{draw_polygon, fill_triangle, fill_circle, draw_circle};
use abrash::shapes::{Polygon, Triangle};
use abrash::math::{Vec2, Mat2};
use abrash::time::FixedTimestep;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF000000;
const GREEN: u32 = 0xFF00FF00;
const RED: u32 = 0xFFFF0000;
const BLUE: u32 = 0xFF0000FF;
const YELLOW: u32 = 0xFFFFFF00;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Abrash - Filled Primitives", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let mut timestep = FixedTimestep::new(60);

    // Create shapes
    let base_triangle = Triangle::new(
        Vec2::new(0.0, -80.0),
        Vec2::new(70.0, 60.0),
        Vec2::new(-70.0, 60.0),
    );
    let base_polygon = Polygon::regular(6, 60.0); // Hexagon

    let tri_center = Vec2::new(200.0, 300.0);
    let hex_center = Vec2::new(400.0, 300.0);
    let circle_center = (600, 300);

    let mut angle: f32 = 0.0;
    let rotation_speed: f32 = 1.5;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle += rotation_speed * timestep.dt();
        }

        framebuffer.clear(BACKGROUND);

        let rotation = Mat2::rotation(angle);

        // Filled rotating triangle (red)
        let tri = base_triangle.transform(&rotation).translate(tri_center);
        fill_triangle(&mut framebuffer, &tri, RED);

        // Wireframe rotating hexagon (green)
        let mut hex = base_polygon.clone();
        hex.transform_in_place(&rotation);
        hex.translate_in_place(hex_center);
        draw_polygon(&mut framebuffer, &hex, GREEN);

        // Pulsing circle (blue filled, yellow outline)
        let pulse = ((angle * 2.0).sin() * 20.0 + 40.0) as i32;
        fill_circle(&mut framebuffer, circle_center.0, circle_center.1, pulse, BLUE);
        draw_circle(&mut framebuffer, circle_center.0, circle_center.1, pulse, YELLOW);

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
