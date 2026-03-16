use abrash::framebuffer::Framebuffer;
use abrash::post_process::dof::{DepthOfFieldConfig, apply_depth_of_field};
use abrash::zbuffer::ZBuffer;

#[test]
fn test_dof_dos() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut fb = Framebuffer::new(100, 100_000).unwrap();
        let zb = ZBuffer::new(100, 100_000).unwrap();
        let config = DepthOfFieldConfig {
            focus_dist: 10.0,
            focus_range: 5.0,
            blur_radius: 1000,
        };
        apply_depth_of_field(&mut fb, &zb, &config);
    }));
    assert!(result.is_ok());
}
