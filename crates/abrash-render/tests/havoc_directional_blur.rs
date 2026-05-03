use proptest::prelude::*;
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::directional_blur::{DirectionalBlurConfig, apply_directional_blur};

proptest! {
    #[test]
    fn havoc_test_directional_blur_overflow(dx in any::<f32>(), dy in any::<f32>(), num_samples in 2usize..100) {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let config = DirectionalBlurConfig { dx, dy, num_samples };
        apply_directional_blur(&mut fb, &config);
    }
}
