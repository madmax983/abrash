use abrash_core::framebuffer::Framebuffer;
use abrash_render::ascii::{AsciiConverter, AsciiCharset};

#[test]
#[ignore]
fn test_capacity_panic() {
    let fb = Framebuffer::new(30000, 30000).unwrap();
    let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
    let _ = converter.to_colored_string();
}
