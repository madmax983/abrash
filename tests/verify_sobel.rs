use abrash::framebuffer::Framebuffer;
use abrash::post_process;

#[test]
fn test_apply_sobel_edge_detection() {
    // Create a 5x5 framebuffer with a white square in the center
    // 0 0 0 0 0
    // 0 1 1 1 0
    // 0 1 1 1 0
    // 0 1 1 1 0
    // 0 0 0 0 0
    let width = 5;
    let height = 5;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFF000000); // Black

    // Draw white square (3x3)
    for y in 1..4 {
        for x in 1..4 {
            fb.set_pixel(x, y, 0xFFFFFFFF);
        }
    }

    // Apply Sobel
    post_process::apply_sobel(&mut fb);

    // Check results
    // The center pixel (2,2) is surrounded by white, so it should have 0 gradient (black).
    let center = fb.get_pixel(2, 2).unwrap();
    assert_eq!(
        center & 0xFFFFFF,
        0,
        "Center pixel should be black (no gradient)"
    );

    // The edge pixels (e.g., 1,1) should have a gradient.
    // Neighbors of (1,1):
    // 0 0 0
    // 0 1 1
    // 0 1 1
    // Top row is all 0. Middle row is 0, 1, 1. Bottom is 0, 1, 1.
    // Sobel X kernel:
    // -1 0 1
    // -2 0 2
    // -1 0 1
    // X Gradient:
    // (-1*0 + 0*0 + 1*0) +
    // (-2*0 + 0*1 + 2*1) +
    // (-1*0 + 0*1 + 1*1)
    // = 0 + 2 + 1 = 3.
    // Wait, let's trace carefully.
    // (1,1) neighborhood:
    // (0,0)=0 (1,0)=0 (2,0)=0
    // (0,1)=0 (1,1)=1 (2,1)=1
    // (0,2)=0 (1,2)=1 (2,2)=1
    //
    // Gx:
    // (-1*0) + (0*0) + (1*0) = 0
    // (-2*0) + (0*1) + (2*1) = 2
    // (-1*0) + (0*1) + (1*1) = 1
    // Sum = 3. Scaled? usually just sum.
    //
    // Gy kernel:
    // -1 -2 -1
    //  0  0  0
    //  1  2  1
    //
    // Gy:
    // (-1*0) + (-2*0) + (-1*0) = 0
    // (0*0)  + (0*1)  + (0*1)  = 0
    // (1*0)  + (2*1)  + (1*1)  = 3
    // Sum = 3.
    //
    // Magnitude = |3| + |3| = 6. (Or sqrt(9+9) = 4.24).
    // If we use luminance 0-255, 1 is actually 255.
    // So Magnitude = 6 * 255 = 1530. Clamped to 255.
    // So it should be white (255).

    let edge_pixel = fb.get_pixel(1, 1).unwrap();
    let r = (edge_pixel >> 16) & 0xFF;
    assert!(r > 0, "Edge pixel should have non-zero gradient");

    // Check a pixel completely outside (0,0) - border handling might leave it 0 or copy it.
    // Usually border pixels are skipped or set to 0.
    let border = fb.get_pixel(0, 0).unwrap();
    assert_eq!(border & 0xFFFFFF, 0, "Border pixel should be black");
}
