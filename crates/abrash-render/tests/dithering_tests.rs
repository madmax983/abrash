use abrash_core::framebuffer::Framebuffer;
use abrash_render::post_process::filters::apply_dithering;

#[test]
fn test_apply_dithering_changes_pixels() {
    let width = 4;
    let height = 4;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a uniform mid-tone grey
    let pixels = fb.as_mut_slice();
    for i in 0..(width * height) {
        pixels[i as usize] = 0xFF_80_80_80;
    }

    apply_dithering(&mut fb);

    // After dithering, the pixels should not all be exactly 0xFF808080.
    // Dithering adds noise based on spatial position.
    let mut has_variance = false;
    let pixels = fb.as_slice();
    for i in 0..(width * height) {
        if pixels[i as usize] != 0xFF_80_80_80 {
            has_variance = true;
            break;
        }
    }

    assert!(
        has_variance,
        "Dithering did not alter the uniform pixel block."
    );
}
