use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;

#[test]
#[should_panic(expected = "Buffer size overflow")]
fn havoc_framebuffer_overflow() {
    // 65536 * 65536 = 4,294,967,296
    // u32::MAX      = 4,294,967,295
    // Result wraps to 0 in release mode if unchecked.
    let width = 65536;
    let height = 65536;
    Framebuffer::new(width, height);
}

#[test]
#[should_panic(expected = "Buffer size overflow")]
fn havoc_zbuffer_overflow() {
    let width = 65536;
    let height = 65536;
    ZBuffer::new(width, height);
}
