use abrash::framebuffer::Framebuffer;
use abrash_render::rasterizer::gouraud::{GouraudSpanStartI64, GouraudGradients};
use abrash::math::{ScreenPoint, Vec3};
use abrash::rasterizer::gouraud::draw_scanline_gouraud;
use abrash::rasterizer::texture::{
    PerspectiveSpanStart, PerspectiveTextureGradients, TexturedGouraudGradients,
    TexturedGouraudSpanStart, draw_scanline_textured_gouraud, draw_scanline_textured_perspective,
};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_scanlines_handle_out_of_bounds_y_gracefully() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // 1. Gouraud
    draw_scanline_gouraud(
        &mut fb,
        &mut zb,
        -1, // Invalid Y
        0,
        50,
        GouraudSpanStartI64 { z_start: 1.0, c_start: (0, 0, 0) },
        &GouraudGradients { dz_dx: 0.0, dc_dx: (0, 0, 0) },
    );
    draw_scanline_gouraud(
        &mut fb,
        &mut zb,
        100, // Invalid Y
        0,
        50,
        GouraudSpanStartI64 { z_start: 1.0, c_start: (0, 0, 0) },
        &GouraudGradients { dz_dx: 0.0, dc_dx: (0, 0, 0) },
    );

    // 2. Texture Perspective
    let texture = Texture::new(32, 32).unwrap();
    let p_start = PerspectiveSpanStart {
        z: 0.0,
        q: 1.0,
        u: 0.0,
        v: 0.0,
    };
    let p_grads = PerspectiveTextureGradients::new(
        ScreenPoint {
            x: 0,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        },
        ScreenPoint {
            x: 10,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        },
        ScreenPoint {
            x: 0,
            y: 10,
            z: 0.0,
            inv_w: 1.0,
        },
        1.0,
        1.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    );

    draw_scanline_textured_perspective(&mut fb, &mut zb, &texture, -1, 0, 50, p_start, &p_grads);
    draw_scanline_textured_perspective(&mut fb, &mut zb, &texture, 100, 0, 50, p_start, &p_grads);

    // 3. Texture Gouraud
    let tg_start = TexturedGouraudSpanStart {
        z: 0.0,
        q: 1.0,
        u: 0.0,
        v: 0.0,
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };
    let (tg_grads, _) = TexturedGouraudGradients::new(
        ScreenPoint {
            x: 0,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        },
        ScreenPoint {
            x: 10,
            y: 0,
            z: 0.0,
            inv_w: 1.0,
        },
        ScreenPoint {
            x: 0,
            y: 10,
            z: 0.0,
            inv_w: 1.0,
        },
        1.0,
        1.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 0.0),
    );

    draw_scanline_textured_gouraud(&mut fb, &mut zb, -1, 0, 50, tg_start, &tg_grads, &texture);
    draw_scanline_textured_gouraud(&mut fb, &mut zb, 100, 0, 50, tg_start, &tg_grads, &texture);
}
