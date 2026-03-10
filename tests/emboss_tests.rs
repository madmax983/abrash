use abrash::framebuffer::Framebuffer;
use abrash::post_process::apply_emboss;

#[test]
fn test_emboss_integration() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFF_FF_FF_FF); // White background

    // Draw a black square in the middle
    for y in 25..75 {
        for x in 25..75 {
            unsafe { fb.set_pixel_unchecked(x, y, 0xFF_00_00_00) };
        }
    }

    apply_emboss(&mut fb);

    // Verify a pixel strictly inside the white area becomes grey
    let p_white = fb.get_pixel(10, 10).unwrap();
    assert_eq!(p_white, 0xFF_80_80_80, "Flat white area should be grey");

    // Verify a pixel strictly inside the black area becomes grey
    let p_black = fb.get_pixel(50, 50).unwrap();
    assert_eq!(p_black, 0xFF_80_80_80, "Flat black area should be grey");

    // Verify an edge pixel is NOT grey (highlighted/shadowed)
    // Left edge of the square is at x=25. The pixel at (24, 50) is white, (25, 50) is black.
    // The emboss kernel [-1 -1 0; -1 0 1; 0 1 1] will definitely produce a non-zero diff here.
    let p_edge = fb.get_pixel(25, 50).unwrap();
    assert_ne!(p_edge, 0xFF_80_80_80, "Edge pixel should not be grey");
}
