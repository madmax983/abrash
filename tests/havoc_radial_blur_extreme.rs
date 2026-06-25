#![cfg(feature = "nova")]
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::radial_blur::apply_radial_blur;

#[test]
#[should_panic(expected = "attempt to add with overflow")]
fn test_havoc_radial_blur_extreme_f32_overflow() {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    // Use an extreme strength that might cause f32 overflow when multiplying by sf_fixed
    let strength = 1e38;
    let cx = 0;
    let cy = 0;
    let samples = 2; // Keep samples low so sf_fixed gets huge

    apply_radial_blur(&mut fb, cx, cy, strength, samples);
}
