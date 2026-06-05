use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::pixel_sort::{PixelSortConfig, apply_pixel_sort};
use proptest::prelude::*;

proptest! {
    #[test]
    fn havoc_pixel_sort_fuzz(
        w in 1..=5000u32,
        h in 1..=5000u32,
        threshold in any::<f32>(),
        vertical in any::<bool>(),
        reverse in any::<bool>(),
    ) {
        if let Ok(mut fb) = Framebuffer::new(w, h) {
            apply_pixel_sort(&mut fb, &PixelSortConfig { threshold, vertical, reverse });
        }
    }
}
