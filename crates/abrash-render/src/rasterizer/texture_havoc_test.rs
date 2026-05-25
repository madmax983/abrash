//! Edge-case bounds testing for the Texture Rasterizer.
use crate::rasterizer::texture::{TexSpanState, TexSpanStep, draw_span_nearest_simd};
use abrash_core::texture::Texture;
use proptest::prelude::*;

#[test]

fn havoc_texture_overflow_simd() {
    let mut fb = vec![0u32; 64];
    let mut zb = vec![100.0f32; 64];
    let mut tex = Texture::new(2, 2).unwrap();
    tex.set_pixel(0, 0, 0x00FF_FFFFFF);

    let z = 1.0;
    let dz_dx = 0.0;

    let u_fix = i32::MAX - 100;
    let v_fix = i32::MAX - 100;
    let du_fix = 286_331_154; // Causes wrap around
    let dv_fix = 286_331_154;

    unsafe {
        draw_span_nearest_simd(
            &mut fb,
            &mut zb,
            &tex,
            TexSpanState { z, u_fix, v_fix },
            TexSpanStep {
                dz_dx,
                du_fix,
                dv_fix,
            },
        );
    }
}

#[test]
fn havoc_texture_simd_buffer_mismatch() {
    let mut fb = vec![0u32; 64];
    let mut zb = vec![100.0f32; 8];
    let mut tex = Texture::new(2, 2).unwrap();
    tex.set_pixel(0, 0, 0x00FF_FFFFFF);

    let z = 1.0;
    let dz_dx = 0.0;
    let u_fix = 0;
    let v_fix = 0;
    let du_fix = 0;
    let dv_fix = 0;

    unsafe {
        draw_span_nearest_simd(
            &mut fb,
            &mut zb,
            &tex,
            TexSpanState { z, u_fix, v_fix },
            TexSpanStep {
                dz_dx,
                du_fix,
                dv_fix,
            },
        );
    }
}

#[test]

fn havoc_texture_simd_zero_fb() {
    let mut fb = vec![0u32; 0];
    let mut zb = vec![100.0f32; 0];
    let mut tex = Texture::new(2, 2).unwrap();
    tex.set_pixel(0, 0, 0x00FF_FFFFFF);

    let z = 1.0;
    let dz_dx = 0.0;
    let u_fix = 0;
    let v_fix = 0;
    let du_fix = 0;
    let dv_fix = 0;

    unsafe {
        draw_span_nearest_simd(
            &mut fb,
            &mut zb,
            &tex,
            TexSpanState { z, u_fix, v_fix },
            TexSpanStep {
                dz_dx,
                du_fix,
                dv_fix,
            },
        );
    }
}
