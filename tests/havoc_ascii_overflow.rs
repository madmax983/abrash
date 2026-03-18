use abrash::ascii::{AsciiCharset, AsciiConverter};
use abrash::framebuffer::Framebuffer;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))] // We don't want to run too many OOM tests

    #[test]
    #[ignore = "👺 Havoc: Intentionally tests an Integer Overflow vulnerability leading to OOM/panic. Will SIGKILL runner if run."]
    fn havoc_ascii_converter_integer_overflow(
        width in 214_748_365u32..214_750_000u32,
        height in 1u32..5u32,
    ) {
        // Area = 214,748,365 * 1 = 214,748,365. Fits in u32, passes validation.
        // RAM usage: 858MB per Framebuffer.
        let fb_res = Framebuffer::new(width, height);

        if let Ok(fb) = fb_res {
            let converter = AsciiConverter::new(&fb, AsciiCharset::Standard);

            // `let mut result = String::with_capacity(((width * 20) * height) as usize);`
            // (214_748_365 * 20) = 4,294,967,300.
            // As u32, 4,294,967,300 wraps around.
            // Then it pushes 214,748,365 * 20 characters into it, causing massive reallocation and likely OOM or timeout.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _s = converter.to_colored_string();
            }));

            prop_assert!(
                result.is_err(),
                "Expected panic due to integer overflow in capacity calculation"
            );
        }
    }
}
