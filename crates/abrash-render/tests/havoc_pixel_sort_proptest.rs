#[cfg(feature = "nova")]
use abrash_core::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use abrash_render::experimental::pixel_sort::{apply_pixel_sort, PixelSortConfig};
#[cfg(feature = "nova")]
use proptest::prelude::*;

#[cfg(feature = "nova")]
proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]

    #[test]
    fn fuzz_apply_pixel_sort(
        w in 1u32..500u32,
        h in 1u32..500u32,
        threshold in any::<f32>(),
        vertical in any::<bool>(),
        reverse in any::<bool>(),
    ) {
        if let Ok(mut fb) = Framebuffer::new(w, h) {
            apply_pixel_sort(&mut fb, &PixelSortConfig { threshold, vertical, reverse });
        }
    }
}
