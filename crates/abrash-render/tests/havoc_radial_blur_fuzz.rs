use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::radial_blur::apply_radial_blur;

#[test]
fn test_havoc_radial_blur_samples_overflow() {
    let mut fb = Framebuffer::new(1, 1).unwrap();
    // The failing input was w=1, h=1, cx=0, cy=61, blur=-521.2831, samples=2 (but actually samples=3 triggered the panic reliably in my earlier test).
    // The inner loop for radial blur uses integer arithmetic for scaling coordinates:
    // `cur_y += step_y` where `step_y` is derived from `-521.2831 * 65536.0` and scaled by `dy`.
    // It overflows i32 after a few samples because the magnitude becomes larger than 2.14 billion.
    apply_radial_blur(&mut fb, 0, 61, -521.2831, 3);
}
