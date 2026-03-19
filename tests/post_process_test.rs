use abrash::framebuffer::Framebuffer;
use abrash::post_process::{BloomConfig, apply_bloom};

#[test]
fn test_apply_bloom_simple() {
    let width = 10;
    let height = 10;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Set a single bright pixel in the center
    // Format: 0xAARRGGBB
    // White pixel with full alpha
    fb.set_pixel(5, 5, 0xFFFF_FFFF);

    // Apply bloom
    // Threshold = 200 (white passes)
    // Blur radius = 2 (should spread to 5 +/- 2 = 3..7)
    // Intensity = 1.0
    let config = BloomConfig {
        threshold: 200,
        blur_radius: 2,
        intensity: 1.0,
    };
    apply_bloom(&mut fb, &config);

    // Check center pixel (should be bright + bloom)
    let center = fb.get_pixel(5, 5).unwrap();
    // It should be white (clamped to 255)
    assert_eq!(center, 0xFFFF_FFFF, "Center pixel should remain white");

    // Check neighbors
    // At (3, 5), it should have received some bloom
    // Original was black (0).
    // Bloom adds light.
    let neighbor = fb.get_pixel(3, 5).unwrap();
    let r = (neighbor >> 16) & 0xFF;
    let g = (neighbor >> 8) & 0xFF;
    let b = neighbor & 0xFF;

    // Radius 2 box blur: kernel size 5x5 = 25 pixels.
    // Center pixel contributes 255 to the blur sum.
    // Average over kernel size?
    // Box blur is separable.
    // Horizontal pass: 1 pixel out of 5 is 255. Avg = 255/5 = 51.
    // Vertical pass: 1 pixel (the row with 51) out of 5 is 51. Avg = 51/5 = 10.
    // So bloom value added should be around 10.
    // Intensity is 1.0. So added value is 10.
    // Pixel was 0. So result should be around 10.
    // (10, 10, 10).

    assert!(r > 0, "Neighbor R should have some bloom");
    assert!(g > 0, "Neighbor G should have some bloom");
    assert!(b > 0, "Neighbor B should have some bloom");

    // Check outside bloom radius
    // At (0, 0), it should be black (too far)
    let far = fb.get_pixel(0, 0).unwrap();
    assert_eq!(far & 0x00FF_FFFF, 0, "Far pixel should be black");
}

#[test]
fn test_apply_bloom_no_change() {
    let width = 10;
    let height = 10;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Set a dim pixel
    fb.set_pixel(5, 5, 0xFF40_4040); // Dark gray (64, 64, 64)

    // Apply bloom with high threshold
    let config = BloomConfig {
        threshold: 200,
        blur_radius: 2,
        intensity: 1.0,
    };
    apply_bloom(&mut fb, &config);

    // Should not have bloomed
    let neighbor = fb.get_pixel(3, 5).unwrap();
    assert_eq!(neighbor & 0x00FF_FFFF, 0, "No bloom expected for dim pixel");
}
