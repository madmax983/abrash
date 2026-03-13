use abrash::framebuffer::Framebuffer;
use abrash::post_process::apply_film_grain;

#[test]
fn test_apply_film_grain_modifies_pixels() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a solid color
    fb.clear(0xFF808080); // Mid-gray

    // Create a copy to compare against
    let mut fb_copy = Framebuffer::new(width, height).unwrap();
    fb_copy.as_mut_slice().copy_from_slice(fb.as_slice());

    // Apply effect
    apply_film_grain(&mut fb, 0.5, 42);

    // Verify that at least some pixels were modified by the noise
    let original_pixels = fb_copy.as_slice();
    let new_pixels = fb.as_slice();

    let mut modified_count = 0;
    for i in 0..(width * height) as usize {
        if original_pixels[i] != new_pixels[i] {
            modified_count += 1;
        }
    }

    assert!(modified_count > 0, "Film grain should modify pixels");
}
