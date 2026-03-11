#![cfg(feature = "nova")]
use abrash::experimental::emboss::apply_emboss;
use abrash::framebuffer::Framebuffer;

#[test]
fn test_apply_emboss() {
    let width = 3;
    let height = 3;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Create a simple gradient/edge pattern
    fb.set_pixel(0, 0, 0xFF00_0000);
    fb.set_pixel(1, 0, 0xFF00_0000);
    fb.set_pixel(2, 0, 0xFF00_0000);

    fb.set_pixel(0, 1, 0xFF00_0000);
    fb.set_pixel(1, 1, 0xFF80_8080); // Center pixel is brighter
    fb.set_pixel(2, 1, 0xFF00_0000);

    fb.set_pixel(0, 2, 0xFF00_0000);
    fb.set_pixel(1, 2, 0xFF00_0000);
    fb.set_pixel(2, 2, 0xFF00_0000);

    // Apply emboss
    apply_emboss(&mut fb);

    // For center pixel (1,1):
    // Kernel:
    // [-1, -1,  0]   [0,   0,   0]
    // [-1,  1,  1] * [0, 128,   0]
    // [ 0,  1,  1]   [0,   0,   0]
    //
    // Sum = (1 * 128) = 128
    // Result = 128 + 128 (bias) = 256 -> clamped to 255.
    // Now it uses average color differences, but here all color channels are the same
    // (RGB for grey are 128, 128, 128)
    //
    // diff_r = 128 - 0 = 128
    // diff_g = 128 - 0 = 128
    // diff_b = 128 - 0 = 128
    // avg_diff = (128 + 128 + 128) / 3 = 128
    // out = (128 + 128).clamp(0, 255) = 255
    // So center pixel should be white (255)

    let p = fb.get_pixel(1, 1).unwrap();
    let r = (p >> 16) & 0xFF;
    let g = (p >> 8) & 0xFF;
    let b = p & 0xFF;
    assert_eq!(r, 255, "Expected bright center pixel due to convolution");
    assert_eq!(g, 255, "Expected bright center pixel due to convolution");
    assert_eq!(b, 255, "Expected bright center pixel due to convolution");
}
