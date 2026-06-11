#![cfg(feature = "nova")]

use abrash::experimental::halftone;
use abrash::framebuffer::Framebuffer;

#[test]
fn test_halftone_filter() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a horizontal grayscale gradient
    for y in 0..height {
        for x in 0..width {
            let intensity = (x as f32 / width as f32 * 255.0) as u32;
            let color = 0xFF00_0000 | (intensity << 16) | (intensity << 8) | intensity;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    // Apply halftone filter
    halftone::apply_halftone(
        &mut fb,
        &abrash::experimental::halftone::HalftoneConfig {
            dot_size: 5.0,
            angle_radians: std::f32::consts::FRAC_PI_4,
        },
    ); // 45 degrees in radians

    // Verify output consists only of black and white pixels
    for y in 0..height {
        for x in 0..width {
            let pixel = fb.get_pixel(x as i32, y as i32).unwrap();
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;

            // Halftone should output either pure black or pure white
            assert!(
                (r == 0 && g == 0 && b == 0) || (r == 255 && g == 255 && b == 255),
                "Pixel at ({x}, {y}) is not black or white: 0x{pixel:08X}"
            );
        }
    }
}
