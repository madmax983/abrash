use abrash_core::framebuffer::Framebuffer;
use abrash_render::ascii::{AsciiConverter, AsciiCharset};
use proptest::prelude::*;

proptest! {
    #[test]
    #[ignore]
    fn test_ascii_capacity_overflow(width in 10000u32..20000u32, height in 10000u32..20000u32) {
        if let Ok(fb) = Framebuffer::new(width, height) {
            let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
            let _ = converter.to_colored_string();
        }
    }
}
