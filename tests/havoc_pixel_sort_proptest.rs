use abrash::experimental::pixel_sort::{PixelSortConfig, apply_pixel_sort};
use abrash::framebuffer::Framebuffer;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_pixel_sort_no_panic(
        w in 1u32..200u32,
        h in 1u32..200u32,
        threshold in 0.0f32..1.0f32,
        vertical in any::<bool>(),
        reverse in any::<bool>(),
    ) {
        if let Ok(mut fb) = Framebuffer::new(w, h) {
            let config = PixelSortConfig { threshold, vertical, reverse };
            apply_pixel_sort(&mut fb, &config);
        }
    }
}
