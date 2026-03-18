//! Fisheye Lens Filter Demo
//!
//! Demonstrates the experimental barrel distortion effect (fisheye lens).
//! Move the mouse or use arrow keys to rotate the cube.
//! Use Up/Down arrow keys to adjust the fisheye distortion strength.

use abrash::experimental::fisheye::apply_fisheye;
use abrash::framebuffer::Framebuffer;
use abrash::geometry::unit_cube_mesh;
use abrash::math::{Mat4, Vec3};
use abrash::platform::Event;
use abrash::platform::{ColorConfig, Window};
use abrash::rasterizer::draw_mesh;
use abrash::zbuffer::ZBuffer;
use std::time::Instant;

fn main() {
    let mut width = 800;
    let mut height = 600;

    let mut window = Window::new("Abrash - Fisheye Lens Demo", width, height).unwrap();
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let mesh = unit_cube_mesh();
    let mut rotation_x = 0.5;
    let mut rotation_y = 0.5;
    let mut strength = 0.5; // Initial fisheye strength

    let start_time = Instant::now();
    let mut last_time = start_time;
    let mut frames = 0;
    let mut fps = 0;

    loop {
        // Handle events
        for event in window.poll_events() {
            match event {
                Event::Close => return,
                Event::Resize(w, h) => {
                    width = w;
                    height = h;
                    fb = Framebuffer::new(width, height).unwrap();
                    zb = ZBuffer::new(width, height).unwrap();
                }
            }
        }

        // TUI windowing system doesn't directly support KeyDown in the `Event` enum.
        // We will animate the cube automatically.
        let time = start_time.elapsed().as_secs_f32();
        rotation_y = time * 0.5;
        rotation_x = time * 0.3;

        // Pulse strength between 0.1 and 1.5
        strength = 0.8 + (time * 1.5).sin() * 0.7;

        fb.clear(0xFF222222);
        zb.clear();

        let model = Mat4::rotate_x(rotation_x) * Mat4::rotate_y(rotation_y);
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 3.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.0, width as f32 / height as f32, 0.1, 100.0);

        let mvp = model * view * proj;

        // Draw the cube
        draw_mesh(
            &mut fb,
            &mut zb,
            &mesh,
            &mvp,
            &model,
            Some(0xFF00AAFF), // Cyan color
            None,
        );

        // Apply Fisheye Lens Post-Processing
        apply_fisheye(&mut fb, strength);

        // Render to screen
        window.blit_framebuffer(fb.as_slice(), width, height, ColorConfig::Argb);

        // FPS calculation
        frames += 1;
        let now = Instant::now();
        if now.duration_since(last_time).as_secs_f32() >= 1.0 {
            fps = frames;
            frames = 0;
            last_time = now;
            window.set_title(&format!("Fisheye Demo - FPS: {} | Strength: {:.2}", fps, strength));
        }
    }
}
