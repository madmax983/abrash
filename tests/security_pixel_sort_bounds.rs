use abrash::experimental::pixel_sort::{apply_pixel_sort, PixelSortConfig};
use abrash::framebuffer::Framebuffer;

#[test]
#[cfg(feature = "nova")]
fn test_pixel_sort_valid_bounds() {
    let mut fb = Framebuffer::new(10, 10).unwrap();
    // Fill with random noise so it actually sorts
    for y in 0..10 {
        for x in 0..10 {
            let color = if (x + y) % 2 == 0 { 0xFFFFFFFF } else { 0xFF000000 };
            fb.set_pixel(x, y, color);
        }
    }

    let config = PixelSortConfig {
        threshold: 0.5,
        vertical: true,
        reverse: false,
    };

    // Should process successfully without tripping the newly added out-of-bounds asserts.
    apply_pixel_sort(&mut fb, &config);

    // We don't verify exact sorting math here, just that it didn't panic.
    assert!(fb.width() == 10 && fb.height() == 10);
}
