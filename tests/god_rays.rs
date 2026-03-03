#![cfg(feature = "nova")]

use abrash::experimental::crepuscular::{GodRaysConfig, apply_god_rays};
use abrash::framebuffer::Framebuffer;

#[test]
fn test_apply_god_rays() {
    let mut fb = Framebuffer::new(32, 32).unwrap();
    fb.clear(0xFF00_0000); // Black

    // Draw a bright "sun" in the center
    for y in 14..18 {
        for x in 14..18 {
            fb.set_pixel(x, y, 0xFFFF_FFFF);
        }
    }

    // Apply effect radiating from the center
    let config = GodRaysConfig {
        light_x: 16.0,
        light_y: 16.0,
        density: 1.0,
        weight: 0.1,
        decay: 0.9,
        exposure: 1.0,
        num_samples: 10,
    };
    apply_god_rays(&mut fb, &config);

    // After the radial blur, pixels outside the sun should no longer be purely black
    // if they are on a line radiating from the center.
    let pixel = fb.get_pixel(10, 16).unwrap();
    // It shouldn't be fully black because of the rays.
    assert!(pixel != 0xFF00_0000);
}
