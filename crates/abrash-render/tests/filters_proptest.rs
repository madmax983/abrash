#![allow(missing_docs)]
use abrash_core::framebuffer::Framebuffer;
use abrash_render::post_process::filters;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]
    #[test]
    fn test_filters_no_crash(w in 1..200u32, h in 1..200u32) {
        let mut fb = Framebuffer::new(w, h).unwrap();
        // Just checking for panics in unsafe simd bounds
        filters::apply_grayscale(&mut fb);
        filters::apply_invert(&mut fb);
        filters::apply_sepia(&mut fb);
    }
}
