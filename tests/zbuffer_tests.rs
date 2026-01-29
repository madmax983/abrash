use abrash::zbuffer::ZBuffer;

#[test]
fn test_zbuffer_new() {
    let zb = ZBuffer::new(100, 100);
    assert_eq!(zb.width(), 100);
    assert_eq!(zb.height(), 100);
    assert_eq!(zb.get_depth(50, 50), Some(f32::INFINITY));
}

#[test]
fn test_zbuffer_test_and_set() {
    let mut zb = ZBuffer::new(100, 100);

    // First write should succeed
    assert!(zb.test_and_set(50, 50, 0.5));
    assert_eq!(zb.get_depth(50, 50), Some(0.5));

    // Closer depth should succeed
    assert!(zb.test_and_set(50, 50, 0.3));
    assert_eq!(zb.get_depth(50, 50), Some(0.3));

    // Farther depth should fail
    assert!(!zb.test_and_set(50, 50, 0.8));
    assert_eq!(zb.get_depth(50, 50), Some(0.3));
}

#[test]
fn test_zbuffer_clear() {
    let mut zb = ZBuffer::new(100, 100);
    zb.test_and_set(50, 50, 0.5);
    zb.clear();
    assert_eq!(zb.get_depth(50, 50), Some(f32::INFINITY));
}

#[test]
fn test_zbuffer_test_and_set_unchecked() {
    let mut zbuffer = ZBuffer::new(10, 10);

    // Test initial set
    let result = unsafe { zbuffer.test_and_set_unchecked(5, 5, 0.5) };
    assert!(result);
    assert_eq!(zbuffer.get_depth(5, 5), Some(0.5));

    // Test update with closer depth
    let result = unsafe { zbuffer.test_and_set_unchecked(5, 5, 0.2) };
    assert!(result);
    assert_eq!(zbuffer.get_depth(5, 5), Some(0.2));

    // Test reject further depth
    let result = unsafe { zbuffer.test_and_set_unchecked(5, 5, 0.8) };
    assert!(!result);
    assert_eq!(zbuffer.get_depth(5, 5), Some(0.2));
}
