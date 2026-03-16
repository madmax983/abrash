use abrash::framebuffer::Framebuffer;
use abrash::math::Mat4;
use abrash::post_process::ssao::{SsaoConfig, apply_ssao};
use abrash::zbuffer::ZBuffer;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_ssao_panic(
        width in 0..100usize,
        height in 0..100usize,
        radius in -100.0..100.0f32,
        bias in -100.0..100.0f32,
        intensity in -100.0..100.0f32,
    ) {
        if let Ok(mut fb) = Framebuffer::new(width as u32, height as u32) {
            if let Ok(zb) = ZBuffer::new(width as u32, height as u32) {
                let proj = Mat4::identity();
                let config = SsaoConfig { radius, bias, intensity };
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    apply_ssao(&mut fb, &zb, &proj, &config);
                }));
            }
        }
    }
}
