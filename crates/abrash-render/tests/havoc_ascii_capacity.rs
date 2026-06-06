use abrash_core::framebuffer::Framebuffer;
use abrash_render::ascii::{AsciiConverter, AsciiCharset};
use proptest::prelude::*;

proptest! {
    #[test]
    #[ignore]
    fn test_capacity_panic_fuzz(width in 15000u32..25000u32, height in 15000u32..25000u32) {
        if let Ok(fb) = Framebuffer::new(width, height) {
            let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
            let _ = converter.to_colored_string();
        }
    }
}
