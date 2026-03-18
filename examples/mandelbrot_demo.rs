use abrash::experimental::mandelbrot::render_mandelbrot;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{Event, Window};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut window = Window::new("Mandelbrot Explorer", 800, 600)?;
    let mut fb = Framebuffer::new(800, 600)?;

    let center_x = -0.5;
    let center_y = 0.0;
    let zoom = 1.0;
    let max_iter = 100;

    let mut needs_redraw = true;
    let mut running = true;

    while window.is_open() && running {
        if needs_redraw {
            fb.clear(0xFF00_0000);
            render_mandelbrot(&mut fb, center_x, center_y, zoom, max_iter);
            window.blit_framebuffer(&fb);
            needs_redraw = false;
        }

        for event in window.poll_events() {
            match event {
                Event::Close => running = false,
                _ => {}
            }
        }
    }

    Ok(())
}
