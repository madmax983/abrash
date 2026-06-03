#![cfg(feature = "nova")]
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::pixel_sort::{PixelSortConfig, apply_pixel_sort};

#[test]
#[should_panic]
fn test_havoc_pixel_sort_zero_width_panic() {
    let width = 0;
    let height = 0;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let config = PixelSortConfig::default();

    // 🧨 The Trigger: `width = 0` causes `chunks_exact_mut(0)` which panics in stdlib!
    apply_pixel_sort(&mut fb, &config);
}
