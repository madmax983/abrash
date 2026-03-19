#![cfg(feature = "nova")]

use abrash::experimental::kuwahara::apply_kuwahara;
use abrash::framebuffer::Framebuffer;

#[test]
fn test_kuwahara_smoothes_noise_preserves_edge() {
    let mut fb = Framebuffer::new(10, 10).unwrap();

    // Create a solid edge down the middle
    // Left side: Red (with noise)
    // Right side: Blue (with noise)
    for y in 0..10 {
        for x in 0..10 {
            if x < 5 {
                // Red side with some noise
                let noise = if y % 2 == 0 { 0x10 } else { 0x00 };
                fb.set_pixel(x, y, 0xFF000000 | ((0x80 + noise) << 16));
            } else {
                // Blue side with some noise
                let noise = if y % 2 == 0 { 0x10 } else { 0x00 };
                fb.set_pixel(x, y, 0xFF000000 | (0x80 + noise));
            }
        }
    }

    // Apply Kuwahara with radius 1 (3x3 window)
    apply_kuwahara(&mut fb, 1);

    // The filter should smooth the noise on each side
    // but preserve the sharp edge between the red and blue sides.

    // Check left side (Red) - should be smoothed
    let left_pixel = fb.get_pixel(2, 5).unwrap();
    let r_left = (left_pixel >> 16) & 0xFF;
    let b_left = left_pixel & 0xFF;
    assert!(
        (0x80..=0x90).contains(&r_left),
        "Left side red channel smoothed"
    );
    assert_eq!(b_left, 0, "Left side should not have blue");

    // Check right side (Blue) - should be smoothed
    let right_pixel = fb.get_pixel(7, 5).unwrap();
    let r_right = (right_pixel >> 16) & 0xFF;
    let b_right = right_pixel & 0xFF;
    assert!(
        (0x80..=0x90).contains(&b_right),
        "Right side blue channel smoothed"
    );
    assert_eq!(r_right, 0, "Right side should not have red");

    // Check the edge itself (x=4 and x=5)
    // The filter should pick the region with the lowest variance,
    // which will be the region entirely on the red side for x=4,
    // and entirely on the blue side for x=5, thus avoiding blending them.
    let edge_left = fb.get_pixel(4, 5).unwrap();
    let r_edge_left = (edge_left >> 16) & 0xFF;
    let b_edge_left = edge_left & 0xFF;
    assert!(r_edge_left >= 0x80, "Edge left side preserved red");
    assert_eq!(b_edge_left, 0, "Edge left side did not bleed blue");

    let edge_right = fb.get_pixel(5, 5).unwrap();
    let r_edge_right = (edge_right >> 16) & 0xFF;
    let b_edge_right = edge_right & 0xFF;
    assert!(b_edge_right >= 0x80, "Edge right side preserved blue");
    assert_eq!(r_edge_right, 0, "Edge right side did not bleed red");
}

#[test]
fn test_kuwahara_preserves_solid_color() {
    let mut fb = Framebuffer::new(10, 10).unwrap();
    fb.clear(0xFF00FF00); // Solid green

    apply_kuwahara(&mut fb, 2);

    for y in 0..10 {
        for x in 0..10 {
            assert_eq!(fb.get_pixel(x, y), Some(0xFF00FF00));
        }
    }
}
