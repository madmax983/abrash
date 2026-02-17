use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::{
    TexturedGouraudGradients, TexturedGouraudSpanStart, draw_scanline_textured_gouraud,
};
use abrash::texture::{FilterMode, Texture};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_draw_scanline_textured_gouraud_nearest(c: &mut Criterion) {
    let width = 200;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tex = Texture::new(256, 256).unwrap();
    // Fill texture with checkerboard
    for y in 0..256 {
        for x in 0..256 {
            let color = if ((x / 16) + (y / 16)) % 2 == 0 {
                0xFFFFFFFF
            } else {
                0xFF000000
            };
            tex.set_pixel(x, y, color);
        }
    }
    tex.filter_mode = FilterMode::Nearest;

    let y = 50;
    let x_start = 10;
    let x_end = 110; // 100 pixels

    // Simple gradients (flat facing camera)
    let gradients = TexturedGouraudGradients {
        dz_dx: 0.001,
        dq_dx: 0.0,         // No perspective distortion (w constant)
        du_dx: 1.0 / 256.0, // traverse 1 pixel per pixel
        dv_dx: 0.0,
        dr_dx: 1.0, // Color changes
        dg_dx: 0.0,
        db_dx: 0.0,
        // Y gradients (unused for single scanline call but needed for struct)
        dq_dy: 0.0,
        du_dy: 0.0,
        dv_dy: 0.0,
        dr_dy: 0.0,
        dg_dy: 0.0,
        db_dy: 0.0,
    };

    let start = TexturedGouraudSpanStart {
        z: 0.5,
        q: 1.0,
        u: 0.0,
        v: 0.0,
        r: 0.0,
        g: 1.0,
        b: 0.0,
    };

    c.bench_function("draw_scanline_textured_gouraud_nearest", |b| {
        b.iter(|| {
            // Reset Z
            let width_usize = width as usize;
            let start_idx = (y as usize) * width_usize + (x_start as usize);
            let end_idx = (y as usize) * width_usize + (x_end as usize);
            for z in &mut zb.as_mut_slice()[start_idx..=end_idx] {
                *z = f32::INFINITY;
            }

            draw_scanline_textured_gouraud(
                &mut fb,
                &mut zb,
                black_box(y),
                black_box(x_start),
                black_box(x_end),
                black_box(start),
                black_box(&gradients),
                black_box(&tex),
            );
        });
    });
}

fn bench_draw_scanline_textured_gouraud_bilinear(c: &mut Criterion) {
    let width = 200;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tex = Texture::new(256, 256).unwrap();
    // Fill texture
    for y in 0..256 {
        for x in 0..256 {
            let color = if ((x / 16) + (y / 16)) % 2 == 0 {
                0xFFFFFFFF
            } else {
                0xFF000000
            };
            tex.set_pixel(x, y, color);
        }
    }
    tex.filter_mode = FilterMode::Bilinear;

    let y = 50;
    let x_start = 10;
    let x_end = 110; // 100 pixels

    let gradients = TexturedGouraudGradients {
        dz_dx: 0.001,
        dq_dx: 0.0,
        du_dx: 1.0 / 256.0,
        dv_dx: 0.0,
        dr_dx: 1.0,
        dg_dx: 0.0,
        db_dx: 0.0,
        dq_dy: 0.0,
        du_dy: 0.0,
        dv_dy: 0.0,
        dr_dy: 0.0,
        dg_dy: 0.0,
        db_dy: 0.0,
    };

    let start = TexturedGouraudSpanStart {
        z: 0.5,
        q: 1.0,
        u: 0.0,
        v: 0.0,
        r: 0.0,
        g: 1.0,
        b: 0.0,
    };

    c.bench_function("draw_scanline_textured_gouraud_bilinear", |b| {
        b.iter(|| {
            let width_usize = width as usize;
            let start_idx = (y as usize) * width_usize + (x_start as usize);
            let end_idx = (y as usize) * width_usize + (x_end as usize);
            for z in &mut zb.as_mut_slice()[start_idx..=end_idx] {
                *z = f32::INFINITY;
            }

            draw_scanline_textured_gouraud(
                &mut fb,
                &mut zb,
                black_box(y),
                black_box(x_start),
                black_box(x_end),
                black_box(start),
                black_box(&gradients),
                black_box(&tex),
            );
        });
    });
}

criterion_group!(
    benches,
    bench_draw_scanline_textured_gouraud_nearest,
    bench_draw_scanline_textured_gouraud_bilinear
);
criterion_main!(benches);
