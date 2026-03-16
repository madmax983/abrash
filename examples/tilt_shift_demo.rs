use abrash::experimental::tilt_shift::{TiltShiftConfig, apply_tilt_shift};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{Event, Window};

fn generate_procedural_city(fb: &mut Framebuffer) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;

    // Draw sky
    for y in 0..height {
        let t = y as f32 / height as f32;
        let r = ((1.0 - t) * 100.0 + t * 200.0) as u32;
        let g = ((1.0 - t) * 150.0 + t * 220.0) as u32;
        let b = ((1.0 - t) * 255.0 + t * 255.0) as u32;
        let sky_col = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        for x in 0..width {
            fb.set_pixel(x, y, sky_col);
        }
    }

    // Draw some fake buildings (rectangles)
    let buildings = [
        (100, 300, 50, 200, 0xFF404040),
        (200, 250, 80, 250, 0xFF606060),
        (350, 150, 60, 350, 0xFF303030),
        (450, 350, 100, 150, 0xFF505050),
        (600, 200, 70, 300, 0xFF707070),
    ];

    for &(bx, by, bw, bh, col) in &buildings {
        fb.clear_rect(bx, by, bw as u32, bh as u32, col);

        // Add fake windows
        for wy in (by + 10..by + bh - 10).step_by(20) {
            for wx in (bx + 10..bx + bw - 10).step_by(15) {
                fb.clear_rect(wx, wy, 8, 12, 0xFFFFFF00); // Yellow lit windows
            }
        }
    }

    // Draw foreground ground/street
    fb.clear_rect(0, 400, width as u32, (height - 400) as u32, 0xFF202020);

    // Draw street lines
    for x in (0..width).step_by(60) {
        fb.clear_rect(x, 480, 30, 10, 0xFFDDDDDD);
    }
}

fn main() {
    let width = 800;
    let height = 600;

    let mut window = Window::new("Abrash - Tilt Shift Demo", width as u32, height as u32).unwrap();
    let mut fb = Framebuffer::new(width as u32, height as u32).unwrap();

    // Configuration for tilt shift effect
    let mut config = TiltShiftConfig {
        focus_dist: 0.5,
        focus_range: 0.1,
        blur_radius: 8,
    };

    let mut running = true;

    println!("Controls:");
    println!("  Up/Down: Move Focus Plane");
    println!("  Left/Right: Change Blur Radius");

    while running {
        for event in window.poll_events() {
            match event {
                Event::Close => running = false,
                _ => {}
            }
        }

        // In a real input system we would check keys here
        // For this simple demo, we just animate the focus point slowly
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f32();
        config.focus_dist = 0.5 + (time * 0.5).sin() * 0.3;

        // Render base image
        generate_procedural_city(&mut fb);

        // Apply effect
        apply_tilt_shift(&mut fb, &config);

        window.blit_framebuffer(&fb);
    }
}
