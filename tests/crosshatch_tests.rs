#![cfg(feature = "nova")]

use abrash::experimental::crosshatch;
use abrash::framebuffer::Framebuffer;

#[test]
fn test_apply_crosshatch_basic() {
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

    // Apply crosshatch filter with spacing 5
    crosshatch::apply_crosshatch(&mut fb, 5);

    // Verify output consists only of black and white pixels (or original colors if we blend, but let's assume black and white for pure crosshatch)
    let mut has_hatching = false;

    for y in 0..height {
        for x in 0..width {
            let pixel = fb.get_pixel(x as i32, y as i32).unwrap();
            let r = (pixel >> 16) & 0xFF;
            let g = (pixel >> 8) & 0xFF;
            let b = pixel & 0xFF;

            // Crosshatch should output either pure black (hatch lines) or pure white (background)
            // Wait, maybe we just draw black lines over the original image, or white background.
            // Let's assume a pure B&W crosshatch effect for simplicity.
            assert!(
                (r == 0 && g == 0 && b == 0) || (r == 255 && g == 255 && b == 255),
                "Pixel at ({}, {}) is not black or white: 0x{:08X}",
                x,
                y,
                pixel
            );

            if r == 0 {
                has_hatching = true;
            }
        }
    }

    assert!(has_hatching, "Filter did not produce any hatching lines");
}
