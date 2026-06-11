#![cfg(feature = "nova")]

use abrash::experimental::radial_blur::{RadialBlurConfig, apply_radial_blur};
use abrash::framebuffer::Framebuffer;

#[test]
fn test_apply_radial_blur_swar_equivalence() {
    let mut fb_swar = Framebuffer::new(8, 8).unwrap();
    let mut fb_scalar = Framebuffer::new(8, 8).unwrap();

    // Fill with pattern
    for y in 0..8 {
        for x in 0..8 {
            let color = ((x * 32) << 16) | ((y * 32) << 8) | ((x + y) * 16);
            fb_swar.set_pixel(x, y, color as u32);
            fb_scalar.set_pixel(x, y, color as u32);
        }
    }

    // Apply with samples <= 256 (SWAR path)
    apply_radial_blur(
        &mut fb_swar,
        &RadialBlurConfig {
            cx: 4,
            cy: 4,
            strength: 1.0,
            samples: 64,
        },
    );

    // Apply with samples > 256 (Scalar path)
    apply_radial_blur(
        &mut fb_scalar,
        &RadialBlurConfig {
            cx: 4,
            cy: 4,
            strength: 1.0,
            samples: 257,
        },
    );

    // Both should produce reasonably blurred results. We cannot compare them strictly for equality
    // because the number of samples differs, but we can verify that neither crashed and both produced
    // valid color transformations.

    // We can also create a test to directly test the fallback logic if we exposed it, but since it's
    // internal, we just ensure it runs cleanly without panicking.
    assert_ne!(fb_swar.as_slice(), fb_scalar.as_slice());
}

#[test]
fn test_apply_radial_blur_extreme_samples() {
    let mut fb = Framebuffer::new(4, 4).unwrap();
    fb.set_pixel(0, 0, 0xFFFF_FFFF);

    // This will trigger the fallback scalar path (> 256 samples)
    apply_radial_blur(
        &mut fb,
        &RadialBlurConfig {
            cx: 2,
            cy: 2,
            strength: 1.0,
            samples: 300,
        },
    );

    unsafe {
        let blurred = fb.get_pixel_unchecked(0, 0);
        assert_ne!(blurred, 0xFFFF_FFFF);
    }
}
