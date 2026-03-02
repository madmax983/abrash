use abrash::framebuffer::Framebuffer;
use abrash::experimental::pixelate::apply_pixelate;

#[test]
fn test_apply_pixelate_basic() {
    let mut fb = Framebuffer::new(4, 4).unwrap();
    fb.clear(0xFFFFFFFF); // White background
    fb.set_pixel(0, 0, 0xFFFF0000); // Red pixel at top-left
    fb.set_pixel(2, 2, 0xFF00FF00); // Green pixel at bottom-right of a 2x2 block

    // Expected block size of 2
    // Top-left block (0,0 to 1,1) should become all red (samples from 0,0)
    // Bottom-right block (2,2 to 3,3) should become all green (samples from 2,2)
    // Other blocks (0,2 to 1,3 and 2,0 to 3,1) should remain white

    apply_pixelate(&mut fb, 2);

    // Top-left block
    assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFF0000);
    assert_eq!(fb.get_pixel(1, 0).unwrap(), 0xFFFF0000);
    assert_eq!(fb.get_pixel(0, 1).unwrap(), 0xFFFF0000);
    assert_eq!(fb.get_pixel(1, 1).unwrap(), 0xFFFF0000);

    // Bottom-right block
    assert_eq!(fb.get_pixel(2, 2).unwrap(), 0xFF00FF00);
    assert_eq!(fb.get_pixel(3, 2).unwrap(), 0xFF00FF00);
    assert_eq!(fb.get_pixel(2, 3).unwrap(), 0xFF00FF00);
    assert_eq!(fb.get_pixel(3, 3).unwrap(), 0xFF00FF00);

    // Top-right block (sampled from 2, 0 which is white)
    assert_eq!(fb.get_pixel(2, 0).unwrap(), 0xFFFFFFFF);
    assert_eq!(fb.get_pixel(3, 0).unwrap(), 0xFFFFFFFF);

    // Bottom-left block (sampled from 0, 2 which is white)
    assert_eq!(fb.get_pixel(0, 2).unwrap(), 0xFFFFFFFF);
    assert_eq!(fb.get_pixel(1, 2).unwrap(), 0xFFFFFFFF);
}

#[test]
fn test_apply_pixelate_odd_size() {
    let mut fb = Framebuffer::new(5, 5).unwrap();
    fb.clear(0xFFFFFFFF); // White
    fb.set_pixel(4, 4, 0xFF0000FF); // Blue at the very bottom right

    apply_pixelate(&mut fb, 3); // 3x3 block size

    // The first block (0,0 to 2,2) should sample 0,0 (White)
    assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFFFFFF);
    assert_eq!(fb.get_pixel(2, 2).unwrap(), 0xFFFFFFFF);

    // The right partial block (3,0 to 4,2) should sample 3,0 (White)
    assert_eq!(fb.get_pixel(3, 0).unwrap(), 0xFFFFFFFF);

    // The bottom-right partial block (3,3 to 4,4) should sample 3,3 (which is White initially, not the blue at 4,4)
    // Wait, the block starts at 3,3. The sample is at 3,3. It was clear to white.
    // So the blue at 4,4 gets overwritten by white from 3,3!
    assert_eq!(fb.get_pixel(4, 4).unwrap(), 0xFFFFFFFF);
}

#[test]
fn test_apply_pixelate_block_size_1() {
    let mut fb = Framebuffer::new(3, 3).unwrap();
    fb.set_pixel(1, 1, 0xFF123456);

    // Size 1 should do nothing
    apply_pixelate(&mut fb, 1);
    assert_eq!(fb.get_pixel(1, 1).unwrap(), 0xFF123456);
}

#[test]
fn test_apply_pixelate_block_size_0() {
    let mut fb = Framebuffer::new(3, 3).unwrap();
    fb.set_pixel(1, 1, 0xFF123456);

    // Size 0 should ideally be handled gracefully (e.g., fallback to 1)
    apply_pixelate(&mut fb, 0);
    assert_eq!(fb.get_pixel(1, 1).unwrap(), 0xFF123456);
}
