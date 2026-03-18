use abrash::post_process::apply_chromatic_aberration;
use abrash::framebuffer::Framebuffer;
use abrash::platform::Window;
use std::time::Instant;

fn main() {
    let width = 800;
    let height = 600;

    let mut window = Window::new("🌟 Nova: Chromatic Aberration Demo", width, height).unwrap();
    let mut fb = Framebuffer::new(width, height).unwrap();

    let mut shift_amount = 0.0f32;
    let mut last_time = Instant::now();

    while window.is_open() {
        let current_time = Instant::now();
        let dt = current_time.duration_since(last_time).as_secs_f32();
        last_time = current_time;

        // Process window events
        window.poll_events();

        // Draw background grid (procedural generation into framebuffer)
        let w = fb.width() as usize;
        let h = fb.height() as usize;
        let pixels = fb.as_mut_slice();

        for y in 0..h {
            for x in 0..w {
                // Draw a simple grid
                let is_grid_line = (x % 50 < 2) || (y % 50 < 2);
                let is_circle = ((x as i32 - w as i32 / 2).pow(2)
                    + (y as i32 - h as i32 / 2).pow(2))
                    < 150_i32.pow(2);

                let color = if is_grid_line {
                    0xFF_AA_AA_AA // Light grey
                } else if is_circle {
                    0xFF_FF_FF_FF // White circle
                } else {
                    0xFF_22_22_22 // Dark grey
                };

                pixels[y * w + x] = color;
            }
        }

        // Oscillate shift amount over time
        shift_amount += dt * 5.0;
        let shift = (shift_amount.sin() * 10.0).abs() as u32;

        apply_chromatic_aberration(&mut fb, shift);

        window.blit_framebuffer(&fb);
    }
}
