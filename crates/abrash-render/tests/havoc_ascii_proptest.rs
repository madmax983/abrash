use abrash_core::framebuffer::Framebuffer;
use abrash_render::ascii::{AsciiCharset, AsciiConverter};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn test_havoc_ascii_colored_string_overflow(w in (u32::MAX / 20) + 1..=(i32::MAX as u32)) {
        // We know that `w * h` must fit in u32 according to Framebuffer::new.
        // If h=0, w * h = 0, which fits, and dimensions <= i32::MAX.
        // So Framebuffer::new(w, 0) is GUARANTEED to succeed.
        // Then AsciiConverter::to_colored_string computes (width * 20), which overflows u32
        // because w > u32::MAX / 20.
        let fb = Framebuffer::new(w, 0).unwrap();
        let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
        let _ = converter.to_colored_string();
    }
}
