#![cfg(feature = "nova")]

use abrash_render::experimental::crt::apply_crt;
use abrash_render::framebuffer::Framebuffer;

#[test]
fn test_apply_crt_distortion() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Create a cross pattern to easily see distortion
    for y in 0..height {
        for x in 0..width {
            let color = if x == 50 || y == 50 {
                0xFFFF_FFFF
            } else {
                0xFF00_0000
            };
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    // Apply CRT distortion
    apply_crt(
        &mut fb,
        &abrash_render::experimental::crt::CrtConfig { distortion: 0.2 },
    ); // moderate distortion

    // Center should remain unaffected
    assert_eq!(
        fb.get_pixel(50, 50),
        Some(0xFFFF_FFFF),
        "Center pixel should not move"
    );

    // The image should be modified (distortion moves pixels)
    let mut changed = false;
    for y in 0..height {
        for x in 0..width {
            let original_color = if x == 50 || y == 50 {
                0xFFFF_FFFF
            } else {
                0xFF00_0000
            };
            if fb.get_pixel(x as i32, y as i32).unwrap() != original_color {
                changed = true;
                break;
            }
        }
    }

    assert!(changed, "Framebuffer should be modified by CRT distortion");
}
