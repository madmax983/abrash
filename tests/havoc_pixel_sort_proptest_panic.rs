use abrash::experimental::pixel_sort::{PixelSortConfig, apply_pixel_sort};
use abrash::framebuffer::Framebuffer;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_pixel_sort_panic(
        w in 0..1000u32,
        h in 0..1000u32,
        threshold in -10.0f32..10.0f32,
        vertical in any::<bool>(),
        reverse in any::<bool>(),
    ) {
        if let Ok(mut fb) = Framebuffer::new(w, h) {
            let config = PixelSortConfig { threshold, vertical, reverse };
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                apply_pixel_sort(&mut fb, &config);
            }));
        }
    }
}
