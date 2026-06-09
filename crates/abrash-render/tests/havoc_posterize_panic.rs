use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::posterize::{apply_posterize, PosterizeConfig};

#[test]
#[ignore = "👹 Havoc: Bypass posterize quantization limit resulting in multiplication overflow panic"]
fn havoc_posterize_panic() {
    let mut fb = Framebuffer::new(1, 1).unwrap();
    fb.set_pixel(0, 0, 0xFFFFFFFF);
    let config = PosterizeConfig { levels: 4294967295.0 }; // Extremely high levels value
    apply_posterize(&mut fb, &config);
}
