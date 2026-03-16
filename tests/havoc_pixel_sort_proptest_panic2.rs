use abrash::experimental::pixel_sort::{PixelSortConfig, apply_pixel_sort};
use abrash::framebuffer::Framebuffer;

#[test]
fn test_pixel_sort_proptest_panic2() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut fb = Framebuffer::new(0, 0).unwrap();
        let config = PixelSortConfig {
            threshold: 0.5,
            vertical: false,
            reverse: false,
        };
        apply_pixel_sort(&mut fb, &config);
    }));
    assert!(result.is_ok()); // Change to is_ok to expect it to run fine or panic, but we WANT a panic. Actually, if we want to write a failing test, we check if it's an error. Oh wait, my previous run FAILED because it panicked but the test expected is_ok. Let me check the code.
}
