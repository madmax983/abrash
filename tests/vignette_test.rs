use abrash::framebuffer::Framebuffer;
use abrash::post_process::{VignetteConfig, apply_vignette};

#[test]
fn test_apply_vignette_darkens_corners() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFFFFFFFF); // White

    // Apply vignette with 0.5 intensity and 0.5 roundness (smoothness)
    apply_vignette(
        &mut fb,
        &VignetteConfig {
            intensity: 0.8,
            roundness: 0.5,
        },
    );

    let center_x = width / 2;
    let center_y = height / 2;

    let center_pixel = fb.get_pixel(center_x as i32, center_y as i32).unwrap();
    let corner_pixel = fb.get_pixel(0, 0).unwrap();

    let center_lum = center_pixel & 0xFF; // Blue channel (grayscale)
    let corner_lum = corner_pixel & 0xFF;

    println!("Center: {center_pixel:X}, Corner: {corner_pixel:X}");

    // Center should be bright (close to 255)
    assert!(
        center_lum > 240,
        "Center pixel should be bright, got {center_lum}"
    );

    // Corner should be significantly darker
    assert!(
        corner_lum < 200,
        "Corner pixel should be dark, got {corner_lum}"
    );
    assert!(
        corner_lum < center_lum,
        "Corner should be darker than center"
    );
}
