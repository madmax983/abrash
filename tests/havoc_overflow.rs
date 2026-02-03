use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;

#[test]
fn havoc_framebuffer_overflow() {
    // 65536 * 65536 = 4,294,967,296
    // u32::MAX      = 4,294,967,295
    // Result wraps to 0 in release mode if unchecked.
    let width = 65536;
    let height = 65536;
    assert!(Framebuffer::new(width, height).is_err());
}

#[test]
fn havoc_zbuffer_overflow() {
    let width = 65536;
    let height = 65536;
    assert!(ZBuffer::new(width, height).is_err());
}
