#![cfg(feature = "nova")]

use abrash::experimental::radial_blur::apply_radial_blur;
use abrash::framebuffer::Framebuffer;

#[test]
fn test_apply_radial_blur_basic() {
    let mut fb = Framebuffer::new(4, 4).unwrap();

    // Fill the framebuffer with an alternating pattern
    for y in 0..4 {
        for x in 0..4 {
            let color = if (x + y) % 2 == 0 {
                0x00FF_FFFF
            } else {
                0x0000_0000
            };
            fb.set_pixel(x, y, color);
        }
    }

    apply_radial_blur(&mut fb, 2, 2, 1.0, 4);

    unsafe {
        // The center pixel shouldn't change (but it will have alpha set to 0xFF)
        assert_eq!(fb.get_pixel_unchecked(2, 2), 0xFFFF_FFFF);

        // Edge pixels should be blurred (no longer pure white or black)
        let edge_pixel = fb.get_pixel_unchecked(0, 1);
        assert_ne!(edge_pixel, 0xFFFF_FFFF);
        assert_ne!(edge_pixel, 0xFF00_0000);
    }
}

#[test]
fn test_apply_radial_blur_no_effect() {
    let mut fb = Framebuffer::new(4, 4).unwrap();
    fb.set_pixel(0, 0, 0xFFFF_0000);

    // Zero strength or zero samples should not modify the image
    apply_radial_blur(&mut fb, 2, 2, 0.0, 4);
    unsafe {
        assert_eq!(fb.get_pixel_unchecked(0, 0), 0xFFFF_0000);
    }

    apply_radial_blur(&mut fb, 2, 2, 0.5, 0);
    unsafe {
        assert_eq!(fb.get_pixel_unchecked(0, 0), 0xFFFF_0000);
    }
}

#[test]
fn test_apply_radial_blur_fixed_point_rounding() {
    let mut fb = Framebuffer::new(5, 5).unwrap();
    fb.set_pixel(0, 0, 0xFFFF_0000); // Red at corner
    fb.set_pixel(2, 2, 0xFF00_FF00); // Green at center

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

#[test]
fn test_apply_radial_blur_oob_center() {
    let mut fb = Framebuffer::new(4, 4).unwrap();
    fb.clear(0xFFFFFFFF);

    // Apply with a center wildly out of bounds
    apply_radial_blur(&mut fb, 100, 100, 1.0, 5);

    unsafe {
        // Since the whole framebuffer was white, the blur from outside should still just sample white
        assert_eq!(fb.get_pixel_unchecked(0, 0), 0xFFFFFFFF);
        assert_eq!(fb.get_pixel_unchecked(3, 3), 0xFFFFFFFF);
    }
}
