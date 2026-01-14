use abrash::framebuffer::Framebuffer;
use abrash::platform::Window;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Abrash - Static Test", 800, 600)?;
    let mut framebuffer = Framebuffer::new(800, 600);

    // Fill with blue
    framebuffer.clear(0xFF0000FF);

    while window.is_open() {
        let _events = window.poll_events();
        window.blit_framebuffer(&framebuffer);
        std::thread::sleep(std::time::Duration::from_millis(16)); // ~60 FPS
    }

    Ok(())
}
