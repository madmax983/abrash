#![cfg(feature = "nova")]
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::radar::{apply_radar, RadarConfig};

#[test]
fn test_radar_effect() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    // Fill with black
    fb.clear(0xFF00_0000);

    let config = RadarConfig {
        center_x: 50,
        center_y: 50,
        radius: 40,
        angle: 0.0,
        trail_length: std::f32::consts::PI / 2.0,
        color: 0xFF00_FF00,
        grid_color: 0xFF00_4400,
    };

    apply_radar(&mut fb, &config);

    // After applying radar, at least some pixels in the radar region should be non-black
    let mut found_non_black = false;
    for y in 0..100 {
        for x in 0..100 {
            if fb.get_pixel(x, y).unwrap() != 0xFF00_0000 {
                found_non_black = true;
                break;
            }
        }
    }
    assert!(found_non_black, "Radar effect did not draw anything");
}
