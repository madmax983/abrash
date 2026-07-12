use abrash_core::blitter::{blit_opaque, SrcRect};
use abrash_core::framebuffer::Framebuffer;
use abrash_core::texture::Texture;

#[test]
#[ignore = "👺 Havoc: Blitter Integer Overflow causes Out-Of-Bounds Panic"]
fn test_blitter_overflow_exploit() {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut tex = Texture::new(10, 10).unwrap();

    let src = SrcRect {
        x: 1,
        y: 0,
        w: u32::MAX, // Triggers sx + w overflow in clip_blit
        h: 1,
    };

    // This will panic inside blit_opaque_unchecked
    blit_opaque(&mut fb, &tex, src, 0, 0);
}
