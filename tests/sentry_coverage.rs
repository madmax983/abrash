use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::{Vec2, Vec3};
use abrash_core::texture::Texture;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::rasterizer::flat::draw_scanline_flat_blended;
use abrash_render::rasterizer::reflection::fill_triangle_reflection;
use abrash_render::rasterizer::texture::fill_triangle_textured;
use abrash_render::skybox::Cubemap;

#[test]
fn test_draw_scanline_flat_blended_basic() {
    let mut fb = Framebuffer::new(32, 32).unwrap();
    let mut zb = ZBuffer::new(32, 32).unwrap();

    draw_scanline_flat_blended(&mut fb, &mut zb, 16, 5, 25, 10.0, 0.1, 0x80FF_0000);

    let pixels = fb.as_slice();
    let mut non_zero = 0;
    for &p in pixels {
        if p != 0xFF00_0000 && p != 0 {
            non_zero += 1;
        }
    }
    assert!(non_zero > 0, "No pixels drawn");
}

#[test]
fn test_draw_scanline_flat_blended_oob_y() {
    let mut fb = Framebuffer::new(32, 32).unwrap();
    let mut zb = ZBuffer::new(32, 32).unwrap();

    for p in fb.as_mut_slice() {
        *p = 0xFF00_0000;
    }

    draw_scanline_flat_blended(&mut fb, &mut zb, -1, 5, 25, 10.0, 0.1, 0x80FF_0000);
    draw_scanline_flat_blended(&mut fb, &mut zb, 32, 5, 25, 10.0, 0.1, 0x80FF_0000);

    let pixels = fb.as_slice();
    for &p in pixels {
        assert_eq!(
            p, 0xFF00_0000,
            "No pixels should be drawn for out-of-bounds Y"
        );
    }
}

#[test]
fn test_draw_scanline_flat_blended_oob_x() {
    let mut fb = Framebuffer::new(32, 32).unwrap();
    let mut zb = ZBuffer::new(32, 32).unwrap();

    draw_scanline_flat_blended(&mut fb, &mut zb, 16, -10, 5, 10.0, 0.1, 0x80FF_0000);
    draw_scanline_flat_blended(&mut fb, &mut zb, 16, 30, 40, 10.0, 0.1, 0x80FF_0000);

    let pixels = fb.as_slice();
    let mut non_zero = 0;
    for &p in pixels {
        if p != 0xFF00_0000 && p != 0 {
            non_zero += 1;
        }
    }
    assert!(non_zero > 0, "Some pixels should be drawn");
}

#[test]
fn test_draw_scanline_flat_blended_transparent() {
    let mut fb = Framebuffer::new(32, 32).unwrap();
    let mut zb = ZBuffer::new(32, 32).unwrap();

    // Draw a solid background
    for p in fb.as_mut_slice() {
        *p = 0xFFFF_FFFF; // White
    }

    draw_scanline_flat_blended(&mut fb, &mut zb, 16, 5, 25, 10.0, 0.1, 0x80FF_0000); // 50% Red

    let pixel = fb.get_pixel(10, 16).unwrap();
    // Since alpha is 0x80 (128), it should be a mix of White and Red
    assert_ne!(pixel, 0xFFFF_FFFF);
    assert_ne!(pixel, 0x80FF_0000);
}

#[test]
fn test_fill_triangle_textured_oob() {
    let mut fb = Framebuffer::new(32, 32).unwrap();
    let mut zb = ZBuffer::new(32, 32).unwrap();

    // Ensure default fb color is set before test
    for p in fb.as_mut_slice() {
        *p = 0xFF00_0000;
    }

    let mut tex = Texture::new(16, 16).unwrap();
    for i in 0..256 {
        tex.pixels_mut()[i] = 0xFFFF_FFFF;
    }

    // Completely off-screen triangle (Y < 0)
    let v0 = ((Vec3::new(0.0, -10.0, 1.0), 1.0), Vec2::new(0.0, 0.0));
    let v1 = ((Vec3::new(-10.0, -20.0, 1.0), 1.0), Vec2::new(1.0, 0.0));
    let v2 = ((Vec3::new(10.0, -20.0, 1.0), 1.0), Vec2::new(0.5, 1.0));

    fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &tex);

    let pixels = fb.as_slice();
    for &p in pixels {
        assert_eq!(
            p, 0xFF00_0000,
            "No pixels should be drawn for out-of-bounds Y"
        );
    }

    // Partially off-screen X
    let v0_x = ((Vec3::new(-10.0, 10.0, 1.0), 1.0), Vec2::new(0.0, 0.0));
    let v1_x = ((Vec3::new(-20.0, -10.0, 1.0), 1.0), Vec2::new(1.0, 0.0));
    let v2_x = ((Vec3::new(10.0, -10.0, 1.0), 1.0), Vec2::new(0.5, 1.0));

    fill_triangle_textured(&mut fb, &mut zb, v0_x, v1_x, v2_x, &tex);

    let mut non_zero = 0;
    for &p in fb.as_slice() {
        if p != 0xFF00_0000 && p != 0 {
            non_zero += 1;
        }
    }
    assert!(
        non_zero > 0,
        "Some pixels should be drawn for partially out-of-bounds X"
    );
}

#[test]
fn test_fill_triangle_reflection_execution() {
    let mut fb = Framebuffer::new(32, 32).unwrap();
    let mut zb = ZBuffer::new(32, 32).unwrap();
    let tex = Texture::new(1, 1).unwrap();
    let cubemap = Cubemap::new([
        tex.clone(),
        tex.clone(),
        tex.clone(),
        tex.clone(),
        tex.clone(),
        tex,
    ]);

    let v0 = (
        (Vec3::new(-10.0, -10.0, -1.0), 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 0.0),
    );
    let v1 = (
        (Vec3::new(10.0, -10.0, -1.0), 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 0.0),
    );
    let v2 = (
        (Vec3::new(0.0, 10.0, -1.0), 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 0.0),
    );

    fill_triangle_reflection(
        &mut fb,
        &mut zb,
        v0,
        v1,
        v2,
        Vec3::new(0.0, 0.0, 0.0),
        &cubemap,
    );
}
