use abrash::experimental::pixel_sort::{PixelSortConfig, apply_pixel_sort};
use abrash::framebuffer::Framebuffer;

#[test]
fn test_pixel_sort_proptest_panic4() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut fb = Framebuffer::new(0, 10).unwrap();
        let config = PixelSortConfig {
            threshold: 0.5,
            vertical: false,
            reverse: false,
        };
        apply_pixel_sort(&mut fb, &config);
    }));
    assert!(result.is_ok());
}
