#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::radial_blur::apply_radial_blur;

#[test]
fn test_havoc_radial_blur_samples_overflow() {
    let mut fb = Framebuffer::new(10, 10).unwrap();

    let _result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        apply_radial_blur(&mut fb, 5, 5, 1_000_000.0, 10_000_000);
    }));
}
