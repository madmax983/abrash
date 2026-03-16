use abrash::framebuffer::Framebuffer;
use abrash::post_process::dof::{DepthOfFieldConfig, apply_depth_of_field};
use abrash::zbuffer::ZBuffer;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_dof_panic(
        width in 0..1000usize,
        height in 0..1000usize,
        focus_dist in -100.0..100.0f32,
        focus_range in -100.0..100.0f32,
        blur_radius in 0..10000u32,
    ) {
        if let Ok(mut fb) = Framebuffer::new(width as u32, height as u32) {
            if let Ok(zb) = ZBuffer::new(width as u32, height as u32) {
                let config = DepthOfFieldConfig { focus_dist, focus_range, blur_radius };
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    apply_depth_of_field(&mut fb, &zb, &config);
                }));
            }
        }
    }
}
