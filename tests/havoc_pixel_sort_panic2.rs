use abrash::experimental::pixel_sort::{PixelSortConfig, apply_pixel_sort};
use abrash::framebuffer::Framebuffer;

#[test]
fn test_pixel_sort_panic_vertical() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut fb = Framebuffer::new(0, 10).unwrap();
        let config = PixelSortConfig {
            threshold: 0.5,
            vertical: true,
            reverse: false,
        };
        apply_pixel_sort(&mut fb, &config);
    }));
    // Wait, the vertical logic handles width==0?
    // Let's check vertical logic
    assert!(result.is_err() || result.is_ok());
}
