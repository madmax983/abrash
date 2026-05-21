use proptest::prelude::*;
use abrash_core::framebuffer::Framebuffer;
use abrash_render::ascii::{AsciiConverter, AsciiCharset};

proptest! {
    #[test]
    #[should_panic]
    fn test_havoc_ascii_zero_width_panic(width in 0..=0u32, height in 0..100u32) {
        if let Ok(fb) = Framebuffer::new(width, height) {
            let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
            let _ = converter.to_colored_string();
        }
    }
}
