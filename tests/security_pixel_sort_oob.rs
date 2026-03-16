#![cfg(feature = "nova")]
use abrash::experimental::pixel_sort::{apply_pixel_sort, PixelSortConfig};
use abrash::framebuffer::Framebuffer;

#[test]
#[should_panic(expected = "Index out of bounds")]
fn test_exploit_pixel_sort_oob() {
    let mut fb = Framebuffer::new(10, 10).unwrap();

    // Use the test-only method to manipulate height, bypassing the constructor's guarantees
    // to simulate a logic bug or memory corruption prior to the call.
    fb.set_height_for_test(11);

    // Force thread_local buffers to be created with valid length before the bug triggers
    let config = PixelSortConfig {
        threshold: 0.0,
        vertical: true,
        reverse: false,
    };

    // In sequential mode, `col_buffer` size is `height`.
    // In parallel mode, Rayon runs. Since `width` is 10, Rayon processes 10 elements.
    // If it reaches `y=10, x=0`, `y * width + x` = 100.
    // The `pixels` length is 100. 100 < 100 is false, so it triggers the panic.

    // Apply pixel sort on the malformed framebuffer. The added `assert!`
    // inside the parallel block will panic.
    apply_pixel_sort(&mut fb, &config);
}
