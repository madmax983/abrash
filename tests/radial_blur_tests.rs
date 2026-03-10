#![cfg(feature = "nova")]

use abrash::experimental::radial_blur::apply_radial_blur;
use abrash::framebuffer::Framebuffer;

#[test]
fn test_apply_radial_blur_basic() {
    let mut fb = Framebuffer::new(4, 4).unwrap();

    // Fill the framebuffer with an alternating pattern
    for y in 0..4 {
        for x in 0..4 {
            let color = if (x + y) % 2 == 0 { 0xFFFFFF } else { 0x000000 };
            fb.set_pixel(x, y, color);
        }
    }

    // Apply radial blur at center (2,2)
    // Strength 0.5 means it will sample up to 50% towards the center.
    // For x=0, y=0, dx=-2, dy=-2.
    // i=0 -> scale=1.0 -> sample at (0,0) -> 0xFFFFFF
    // i=1 -> scale=0.833 -> sample at (0.33,0.33) -> (0,0) -> 0xFFFFFF
    // i=2 -> scale=0.666 -> sample at (0.66,0.66) -> (0,0) -> 0xFFFFFF
    // i=3 -> scale=0.5 -> sample at (1,1) -> (1,1) -> 0xFFFFFF
    // Ah, our checkerboard has 0xFFFFFF at (0,0) and (1,1) (since 0+0=0, 1+1=2).
    // Let's sample a different point that will cross different colors.
    apply_radial_blur(&mut fb, 2, 2, 1.0, 4);

    unsafe {
        // The center pixel shouldn't change
        assert_eq!(fb.get_pixel_unchecked(2, 2), 0xFFFFFF);

        // Edge pixels should be blurred (no longer pure white or black)
        // For (0,1), it will sample from (0,1) towards (2,2).
        // (0,1) is 0x000000
        // (1,1) is 0xFFFFFF
        let edge_pixel = fb.get_pixel_unchecked(0, 1);
        assert_ne!(edge_pixel, 0xFFFFFF);
        assert_ne!(edge_pixel, 0x000000);
    }
}

#[test]
fn test_apply_radial_blur_no_effect() {
    let mut fb = Framebuffer::new(4, 4).unwrap();
    fb.set_pixel(0, 0, 0xFF0000);

    // Zero strength or zero samples should not modify the image
    apply_radial_blur(&mut fb, 2, 2, 0.0, 4);
    unsafe {
        assert_eq!(fb.get_pixel_unchecked(0, 0), 0xFF0000);
    }

    apply_radial_blur(&mut fb, 2, 2, 0.5, 0);
    unsafe {
        assert_eq!(fb.get_pixel_unchecked(0, 0), 0xFF0000);
    }
}

#[test]
fn test_apply_radial_blur_fixed_point_rounding() {
    let mut fb = Framebuffer::new(5, 5).unwrap();
    fb.set_pixel(0, 0, 0xFF0000); // Red at corner
    fb.set_pixel(2, 2, 0x00FF00); // Green at center

    // With 5 samples and strength 1.0 from (0,0) to (2,2)
    // The samples should be (0,0), (0.5,0.5)->(0,0), (1,1), (1.5,1.5)->(1,1), (2,2)
    // Wait, with 5 samples, strength 1.0:
    // i=0: t=0 -> scale=1.0 -> (0,0) -> Red
    // i=1: t=0.25 -> scale=0.75 -> dx=-2, dy=-2 -> sample_x = 2 + (-2)*0.75 = 0.5 -> 0 -> Red
    // i=2: t=0.50 -> scale=0.50 -> dx=-2, dy=-2 -> sample_x = 2 + (-2)*0.50 = 1.0 -> 1 -> Black
    // i=3: t=0.75 -> scale=0.25 -> dx=-2, dy=-2 -> sample_x = 2 + (-2)*0.25 = 1.5 -> 1 -> Black
    // i=4: t=1.00 -> scale=0.00 -> dx=-2, dy=-2 -> sample_x = 2 + (-2)*0.00 = 2.0 -> 2 -> Green

    apply_radial_blur(&mut fb, 2, 2, 1.0, 5);

    unsafe {
        let blurred = fb.get_pixel_unchecked(0, 0);
        // It should have some red and some green, but no other colors
        let r = (blurred >> 16) & 0xFF;
        let g = (blurred >> 8) & 0xFF;
        let b = blurred & 0xFF;

        assert!(r > 0, "Should contain red from (0,0)");
        assert!(g > 0, "Should contain green from (2,2)");
        assert_eq!(b, 0, "Should not contain blue");
    }
}
