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
