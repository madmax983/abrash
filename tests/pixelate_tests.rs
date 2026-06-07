#![cfg(feature = "nova")]
use abrash::experimental::pixelate::apply_pixelate;
use abrash::framebuffer::Framebuffer;

#[test]
fn test_apply_pixelate_basic() {
    let mut fb = Framebuffer::new(4, 4).unwrap();

    // Fill with distinct colors per row
    for y in 0..4 {
        for x in 0..4 {
            fb.set_pixel(x, y, (y * 10) as u32);
        }
    }

    apply_pixelate(
        &mut fb,
        &abrash::experimental::pixelate::PixelateConfig { block_size: 2 },
    );

    unsafe {
        // Block 1: (0,0) - (1,1) -> Should take color of (0,0) which is 0
        assert_eq!(fb.get_pixel_unchecked(0, 0), 0);
        assert_eq!(fb.get_pixel_unchecked(1, 0), 0);
        assert_eq!(fb.get_pixel_unchecked(0, 1), 0);
        assert_eq!(fb.get_pixel_unchecked(1, 1), 0);

        // Block 2: (2,0) - (3,1) -> Should take color of (2,0) which is 0
        assert_eq!(fb.get_pixel_unchecked(2, 0), 0);
        assert_eq!(fb.get_pixel_unchecked(3, 0), 0);
        assert_eq!(fb.get_pixel_unchecked(2, 1), 0);
        assert_eq!(fb.get_pixel_unchecked(3, 1), 0);

        // Block 3: (0,2) - (1,3) -> Should take color of (0,2) which is 20
        assert_eq!(fb.get_pixel_unchecked(0, 2), 20);
        assert_eq!(fb.get_pixel_unchecked(1, 2), 20);
        assert_eq!(fb.get_pixel_unchecked(0, 3), 20);
        assert_eq!(fb.get_pixel_unchecked(1, 3), 20);
    }
}

#[test]
fn test_apply_pixelate_no_effect() {
    let mut fb = Framebuffer::new(2, 2).unwrap();
    fb.set_pixel(0, 0, 10);
    fb.set_pixel(1, 1, 20);

    // size 1 does nothing
    apply_pixelate(
        &mut fb,
        &abrash::experimental::pixelate::PixelateConfig { block_size: 1 },
    );
    unsafe {
        assert_eq!(fb.get_pixel_unchecked(0, 0), 10);
        assert_eq!(fb.get_pixel_unchecked(1, 1), 20);
    }
}

#[test]
fn test_apply_pixelate_edge_case() {
    let mut fb = Framebuffer::new(3, 3).unwrap();
    fb.set_pixel(0, 0, 10);
    fb.set_pixel(2, 2, 20);

    // Block size 2 on a 3x3 buffer.
    // Rightmost and bottommost blocks will be truncated.
    apply_pixelate(
        &mut fb,
        &abrash::experimental::pixelate::PixelateConfig { block_size: 2 },
    );

    unsafe {
        assert_eq!(fb.get_pixel_unchecked(0, 0), 10);
        assert_eq!(fb.get_pixel_unchecked(1, 1), 10);

        // The pixel at (2,0) determines the right block
        let top_right = fb.get_pixel_unchecked(2, 0);
        assert_eq!(fb.get_pixel_unchecked(2, 1), top_right);

        // The pixel at (0,2) determines the bottom block
        let bottom_left = fb.get_pixel_unchecked(0, 2);
        assert_eq!(fb.get_pixel_unchecked(1, 2), bottom_left);

        // The pixel at (2,2) forms a 1x1 block
        assert_eq!(fb.get_pixel_unchecked(2, 2), 20);
    }
}
