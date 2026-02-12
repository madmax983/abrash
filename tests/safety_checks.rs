use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;

#[test]
fn framebuffer_new_limits() {
    // Valid dimensions
    assert!(Framebuffer::new(100, 100).is_ok());

    // Too large dimensions (exceeds i32::MAX)
    assert!(Framebuffer::new(i32::MAX as u32 + 1, 100).is_err());
    assert!(Framebuffer::new(100, i32::MAX as u32 + 1).is_err());

    // Overflow (width * height > u32::MAX)
    // 65536 * 65536 = 4294967296 (u32::MAX + 1)
    let w = 65536;
    let h = 65536;
    assert!(Framebuffer::new(w, h).is_err());
}

#[test]
fn framebuffer_pixel_access() {
    let mut fb = Framebuffer::new(100, 100).unwrap();

    // Set valid pixel
    fb.set_pixel(10, 10, 0xFFFFFFFF);
    assert_eq!(fb.get_pixel(10, 10), Some(0xFFFFFFFF));

    // Set invalid pixel (negative)
    fb.set_pixel(-1, 10, 0xFF000000);
    // Should not panic, and get_pixel should return None
    assert_eq!(fb.get_pixel(-1, 10), None);

    // Set invalid pixel (too large x)
    fb.set_pixel(100, 10, 0xFF000000);
    assert_eq!(fb.get_pixel(100, 10), None);

    // Set invalid pixel (too large y)
    fb.set_pixel(10, 100, 0xFF000000);
    assert_eq!(fb.get_pixel(10, 100), None);
}

#[test]
fn framebuffer_unsafe_access() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let color = 0xFFAABBCC;

    // Safety: 50, 50 is within bounds (100x100)
    unsafe {
        fb.set_pixel_unchecked(50, 50, color);
        assert_eq!(fb.get_pixel_unchecked(50, 50), color);
    }
}

#[test]
fn zbuffer_new_limits() {
    // Valid dimensions
    assert!(ZBuffer::new(100, 100).is_ok());

    // Too large dimensions
    assert!(ZBuffer::new(i32::MAX as u32 + 1, 100).is_err());

    // Overflow
    let w = 65536;
    let h = 65536;
    assert!(ZBuffer::new(w, h).is_err());
}

#[test]
fn zbuffer_access() {
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Valid access
    // Default is INFINITY
    assert_eq!(zb.get_depth(10, 10), Some(f32::INFINITY));

    assert!(zb.test_and_set(10, 10, 0.5));
    assert_eq!(zb.get_depth(10, 10), Some(0.5));

    // Closer depth should update
    assert!(zb.test_and_set(10, 10, 0.4));
    assert_eq!(zb.get_depth(10, 10), Some(0.4));

    // Further depth should fail and not update
    assert!(!zb.test_and_set(10, 10, 0.6));
    assert_eq!(zb.get_depth(10, 10), Some(0.4));

    // Invalid access (out of bounds)
    assert!(!zb.test_and_set(-1, 10, 0.5));
    assert_eq!(zb.get_depth(-1, 10), None);

    assert!(!zb.test_and_set(100, 10, 0.5));
    assert_eq!(zb.get_depth(100, 10), None);

    assert!(!zb.test_and_set(10, 100, 0.5));
    assert_eq!(zb.get_depth(10, 100), None);
}

#[test]
fn zbuffer_unsafe_access() {
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Safety: 50, 50 is within bounds
    unsafe {
        assert!(zb.test_and_set_unchecked(50, 50, 0.5));
        assert_eq!(zb.get_depth_unchecked(50, 50), 0.5);
    }
}
