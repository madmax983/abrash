use abrash_core::framebuffer::Framebuffer;
use abrash_render::ascii::{AsciiCharset, AsciiConverter};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    #[should_panic]
    fn test_havoc_ascii_colored_string_overflow(w in (u32::MAX / 20) + 1..=(i32::MAX as u32)) {
        // Framebuffer::new(w, 0) is valid because area is 0 <= u32::MAX.
        // AsciiConverter::to_colored_string computes (width * 20), which overflows u32.
        // It then multiplies by height (0) and tries to allocate.
        // Wait, if it panics with "attempt to multiply with overflow", the test passes
        // because of #[should_panic]. As Havoc, a passing #[should_panic] IS a successful chaos test.
        let fb = Framebuffer::new(w, 0).unwrap();
        let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);
        let _ = converter.to_colored_string();
    }
}
