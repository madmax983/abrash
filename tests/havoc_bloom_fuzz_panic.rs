use abrash::framebuffer::Framebuffer;
use abrash::post_process::bloom::{BloomConfig, apply_bloom};
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_bloom_panic(
        width in 0..1000usize,
        height in 0..1000usize,
        threshold in 0..255u8,
        blur_radius in 0..1000u32,
        intensity in -100.0..100.0f32,
    ) {
        if let Ok(mut fb) = Framebuffer::new(width as u32, height as u32) {
            let config = BloomConfig { threshold, blur_radius, intensity };
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                apply_bloom(&mut fb, &config);
            }));
        }
    }
}
