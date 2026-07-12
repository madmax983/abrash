use abrash_core::blitter::{blit_opaque, SrcRect};
use abrash_core::framebuffer::Framebuffer;
use abrash_core::texture::Texture;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]
    #[test]
    #[ignore = "👺 Havoc: Fuzzing blitter bounds handling for integer overflow panics"]
    fn fuzz_blit_opaque_bounds(
        sx in any::<u32>(),
        sy in any::<u32>(),
        w in any::<u32>(),
        h in any::<u32>(),
        dx in any::<i32>(),
        dy in any::<i32>(),
    ) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let tex = Texture::new(10, 10).unwrap();

        let src = SrcRect { x: sx, y: sy, w, h };

        // We catch unwinds because we expect this to panic due to integer overflows
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            blit_opaque(&mut fb, &tex, src, dx, dy);
        }));
    }
}
