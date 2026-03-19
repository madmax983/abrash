use abrash::framebuffer::Framebuffer;
use abrash::post_process::{BloomConfig, apply_bloom};

#[test]
fn test_apply_bloom_effect() {
    let width = 5;
    let height = 5;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Set center pixel to bright white
    fb.set_pixel(2, 2, 0xFFFF_FFFF);

    // Apply bloom with a radius of 1
    let config = BloomConfig {
        threshold: 100,
        blur_radius: 1,
        intensity: 1.0,
    };
    apply_bloom(&mut fb, &config);

    // Center pixel should remain bright (or brighter)
    let center = fb.get_pixel(2, 2).unwrap();
    assert_eq!(center, 0xFFFF_FFFF, "Center pixel should remain white");

    // Neighbor (2, 1) should be brighter than black (0xFF000000)
    let neighbor = fb.get_pixel(2, 1).unwrap();
    let neighbor_r = (neighbor >> 16) & 0xFF;
    assert!(
        neighbor_r > 0,
        "Neighbor pixel should have some brightness due to bloom"
    );
}
