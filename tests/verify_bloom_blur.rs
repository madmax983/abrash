use abrash::framebuffer::Framebuffer;
use abrash::post_process::apply_bloom;

#[test]
fn test_verify_bloom_blur() {
    let width = 8;
    let height = 8;
    let mut fb = Framebuffer::new(width, height).unwrap();
    // Single bright pixel in middle
    fb.set_pixel(4, 4, 0xFFFFFFFF);

    // Apply Bloom
    // radius = 2
    // intensity = 1.0
    // threshold = 0
    apply_bloom(&mut fb, 0, 2, 1.0);

    // Check that blur spread horizontally and vertically
    // Original pixel at 4,4 should be bright.
    // Neighbors should be non-black.
    // Center
    let p = fb.get_pixel(4, 4).unwrap();
    assert_ne!(p & 0xFFFFFF, 0, "Center should be lit");

    // Horizontal neighbor (2 pixels away)
    let p_h = fb.get_pixel(6, 4).unwrap();
    assert_ne!(p_h & 0xFFFFFF, 0, "Horizontal neighbor should be lit");

    // Vertical neighbor
    let p_v = fb.get_pixel(4, 6).unwrap();
    assert_ne!(p_v & 0xFFFFFF, 0, "Vertical neighbor should be lit");

    // Corner (4+2, 4+2) = (6,6)
    // Box blur is separable, so corners are filled.
    let p_c = fb.get_pixel(6, 6).unwrap();
    assert_ne!(p_c & 0xFFFFFF, 0, "Corner neighbor should be lit");
}
