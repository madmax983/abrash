use crate::rasterizer::texture::draw_span_nearest_simd;
use abrash_core::texture::Texture;
use proptest::prelude::*;

#[test]
#[ignore = "👹 Havoc: Trigger SIMD out of bounds access directly"]
fn havoc_texture_overflow_simd() {
    let mut fb = vec![0u32; 64];
    let mut zb = vec![100.0f32; 64];
    let mut tex = Texture::new(2, 2).unwrap();
    tex.set_pixel(0, 0, 0xFFFFFFFF);

    let z = 1.0;
    let dz_dx = 0.0;

    let u_fix = i32::MAX - 100;
    let v_fix = i32::MAX - 100;
    let du_fix = 286331154; // Causes wrap around
    let dv_fix = 286331154;

    unsafe {
        draw_span_nearest_simd(
            &mut fb, &mut zb, &tex, z, dz_dx, u_fix, v_fix, du_fix, dv_fix,
        );
    }
}

#[test]
#[ignore = "👹 Havoc: Trigger out of bounds access by mismatched buffer lengths"]
fn havoc_texture_simd_buffer_mismatch() {
    let mut fb = vec![0u32; 64];
    let mut zb = vec![100.0f32; 8];
    let mut tex = Texture::new(2, 2).unwrap();
    tex.set_pixel(0, 0, 0xFFFFFFFF);

    let z = 1.0;
    let dz_dx = 0.0;
    let u_fix = 0;
    let v_fix = 0;
    let du_fix = 0;
    let dv_fix = 0;

    unsafe {
        draw_span_nearest_simd(
            &mut fb, &mut zb, &tex, z, dz_dx, u_fix, v_fix, du_fix, dv_fix,
        );
    }
}

#[test]
#[ignore = "👹 Havoc: Trigger SIMD zero length frame buffer"]
fn havoc_texture_simd_zero_fb() {
    let mut fb = vec![0u32; 0];
    let mut zb = vec![100.0f32; 0];
    let mut tex = Texture::new(2, 2).unwrap();
    tex.set_pixel(0, 0, 0xFFFFFFFF);

    let z = 1.0;
    let dz_dx = 0.0;
    let u_fix = 0;
    let v_fix = 0;
    let du_fix = 0;
    let dv_fix = 0;

    unsafe {
        draw_span_nearest_simd(
            &mut fb, &mut zb, &tex, z, dz_dx, u_fix, v_fix, du_fix, dv_fix,
        );
    }
}
