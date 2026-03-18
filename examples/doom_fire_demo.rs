use abrash::framebuffer::Framebuffer;
use abrash::platform::{Event, Window};
use abrash::experimental::doom_fire::DoomFire;
use std::time::Instant;

fn main() {
    let width = 640;
    let height = 480;

    // To get the classic chunky pixels look, we simulate the fire at half resolution
    // (or even smaller) and then we'll just draw it centered on the bottom.
    // Or we can simulate it across the whole width but less height.
    let fire_width = (width / 2) as usize;
    let fire_height = (height / 3) as usize;

    println!("🔥 Doom Fire Effect Demo");
    println!("Controls: Close window to exit");

    // Initialize Window
    let mut window = Window::new("Doom Fire Demo", width, height).unwrap();
    let mut fb = Framebuffer::new(width as u32, height as u32).unwrap();

    let mut fire = DoomFire::new(fire_width, fire_height);

    let target_frame_time = std::time::Duration::from_millis(1000 / 30); // 30 FPS fire

    loop {
        let frame_start = Instant::now();

        // Handle events
        for event in window.poll_events() {
            match event {
                Event::Close => return,
                _ => {}
            }
        }

        // 1. Clear background
        fb.clear(0xFF_070707); // Dark black/grey

        // 2. Update Fire
        fire.update();

        // 3. Draw Fire
        // Center horizontally, stick to bottom
        let offset_x = (width as i32 - fire_width as i32) / 2;
        let offset_y = height as i32 - fire_height as i32;
        fire.draw(&mut fb, offset_x, offset_y);

        // 4. Blit to screen
        window.blit_framebuffer(&fb);

        // Cap framerate so the fire doesn't move too fast
        let elapsed = frame_start.elapsed();
        if elapsed < target_frame_time {
            std::thread::sleep(target_frame_time - elapsed);
        }
    }
}
