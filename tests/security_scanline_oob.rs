use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::texture::{
    PerspectiveSpanStart, PerspectiveTextureGradients, draw_scanline_textured_perspective,
};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use abrash::math::ScreenPoint;

#[test]
fn test_draw_scanline_textured_perspective_oob() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();
    let tex = Texture::new(2, 2).unwrap();

    let start = PerspectiveSpanStart {
        z: 1.0,
        q: 1.0,
        u: 0.0,
        v: 0.0,
    };

    let gradients = PerspectiveTextureGradients {
        dz_dx: 0.0,
        dq_dx: 0.0,
        du_dx: 0.0,
        dv_dx: 0.0,
        dq_dy: 0.0,
        du_dy: 0.0,
        dv_dy: 0.0,
    };

    // Extreme negative x_start should cause diff to overflow i32 if not handled properly.
    draw_scanline_textured_perspective(
        &mut fb, &mut zb, &tex, 5, // y
        std::i32::MIN, // x_start
        50, // x_end
        start, &gradients,
    );
}

#[test]
fn test_draw_scanline_flat_oob() {
    use abrash::rasterizer::draw_scanline_flat;
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Extreme negative x_start should cause diff to overflow i32 if not handled properly.
    draw_scanline_flat(
        &mut fb, &mut zb, 5, // y
        std::i32::MIN, // x_start
        50, // x_end
        1.0, // z_start
        0.0, // dz_dx
        0xFFFFFFFF, // color
    );
}
