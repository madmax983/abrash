#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::lens_distortion::{apply_lens_distortion, LensDistortionConfig};

#[test]
fn test_lens_distortion_changes_buffer() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill the framebuffer with a simple grid pattern
    for y in 0..height {
        for x in 0..width {
            let color = if (x % 10 == 0) || (y % 10 == 0) {
                0xFFFFFFFF // White lines
            } else {
                0xFF000000 // Black background
            };
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    let original_fb = fb.as_slice().to_vec();

    let config = LensDistortionConfig {
        distortion: 0.5,
        scale: 0.8,
    };

    apply_lens_distortion(&mut fb, config);

    // Verify that the buffer has been changed
    let mut changed = false;
    for i in 0..(width * height) as usize {
        if fb.as_slice()[i] != original_fb[i] {
            changed = true;
            break;
        }
    }

    assert!(changed, "Lens Distortion Filter should alter the framebuffer");
}

#[test]
fn test_lens_distortion_no_panic_extreme_inputs() {
    let mut fb = Framebuffer::new(10, 10).unwrap();
    fb.clear(0xFFFFFFFF);

    // Extreme pincushion
    apply_lens_distortion(
        &mut fb,
        LensDistortionConfig {
            distortion: 100.0,
            scale: 1.0,
        },
    );

    // Extreme barrel
    apply_lens_distortion(
        &mut fb,
        LensDistortionConfig {
            distortion: -100.0,
            scale: 1.0,
        },
    );

    // Extreme scale
    apply_lens_distortion(
        &mut fb,
        LensDistortionConfig {
            distortion: 0.5,
            scale: 100.0,
        },
    );

    // Should not panic, and out-of-bounds sampling should result in black pixels
}
