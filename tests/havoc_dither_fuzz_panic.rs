use abrash::experimental::dither::{DitherConfig, DitherMode, apply_dither};
use abrash::framebuffer::Framebuffer;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_dither_panic(
        width in 0..1000usize,
        height in 0..1000usize,
        color_depth in 0..255u8,
    ) {
        if let Ok(mut fb) = Framebuffer::new(width as u32, height as u32) {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                apply_dither(&mut fb, DitherConfig { mode: DitherMode::FloydSteinberg, color_depth });
                apply_dither(&mut fb, DitherConfig { mode: DitherMode::Ordered2x2, color_depth });
                apply_dither(&mut fb, DitherConfig { mode: DitherMode::Ordered4x4, color_depth });
                apply_dither(&mut fb, DitherConfig { mode: DitherMode::Ordered8x8, color_depth });
            }));
        }
    }
}
