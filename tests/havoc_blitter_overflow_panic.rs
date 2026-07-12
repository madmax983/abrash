use abrash_core::blitter::{blit_opaque, SrcRect};
use abrash_core::framebuffer::Framebuffer;
use abrash_core::texture::Texture;

#[test]
#[ignore = "👺 Havoc: Blitter Integer Overflow causes Panic in clip_blit"]
fn test_blitter_overflow_exploit() {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let tex = Texture::new(10, 10).unwrap();

    let src = SrcRect {
        x: u32::MAX - 5,
        y: 0,
        w: 10,
        h: 1,
    };

    // This will panic inside clip_blit: `sx + w > tex_w` -> `attempt to add with overflow`
    blit_opaque(&mut fb, &tex, src, 0, 0);
}
