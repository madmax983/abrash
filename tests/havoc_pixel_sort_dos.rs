use abrash::experimental::pixel_sort::{PixelSortConfig, apply_pixel_sort};
use abrash::framebuffer::Framebuffer;

#[test]
fn test_pixel_sort_dos() {
    let mut fb = Framebuffer::new(1_000, 10).unwrap();
    let config = PixelSortConfig {
        threshold: 0.5,
        vertical: true,
        reverse: false,
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        apply_pixel_sort(&mut fb, &config);
    }));
    assert!(result.is_ok());
}
