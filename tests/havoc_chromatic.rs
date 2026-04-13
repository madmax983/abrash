use abrash::framebuffer::Framebuffer;
use abrash::post_process::filters::apply_chromatic_aberration;

#[test]
fn test_chromatic_aberration_out_of_bounds() {
    let mut fb = Framebuffer::new(10, 10).unwrap();
    // Large offset > width
    apply_chromatic_aberration(
        &mut fb,
        &abrash::post_process::ChromaticAberrationConfig { offset: 100 },
    );
}
