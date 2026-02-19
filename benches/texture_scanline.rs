use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::{
    PerspectiveSpanStart, PerspectiveTextureGradients, draw_scanline_textured_perspective,
};
use abrash::texture::{FilterMode, Texture};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_scanline_lengths(c: &mut Criterion) {
    let width = 2000;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tex = Texture::new(256, 256).unwrap();
    // Fill with pattern
    for y in 0..256 {
        for x in 0..256 {
            tex.set_pixel(x, y, (x ^ y) as u32 | 0xFF000000);
        }
    }
    // Use Bilinear to stress the SIMD path more (if it supports it)
    tex.filter_mode = FilterMode::Bilinear;

    let lengths = [16, 32, 64, 100, 500, 1920];

    // Gradients simulating looking at a floor plane at an angle
    let gradients = PerspectiveTextureGradients {
        dz_dx: 0.0001,
        dq_dx: 0.00005, // Some perspective
        du_dx: 0.005,
        dv_dx: 0.005,
        dq_dy: 0.0,
        du_dy: 0.0,
        dv_dy: 0.0,
    };

    let start = PerspectiveSpanStart {
        z: 0.5,
        q: 1.0,
        u: 0.0,
        v: 0.0,
    };

    let y = 50;
    let x_start = 0;

    let mut group = c.benchmark_group("scanline_perspective_bilinear");

    for &len in &lengths {
        group.bench_function(format!("len_{}", len), |b| {
            b.iter(|| {
                let x_end = x_start + len;
                // Clear Z-buffer slice
                let start_idx = (y as usize) * (width as usize) + (x_start as usize);
                let end_idx = (y as usize) * (width as usize) + (x_end as usize);
                zb.as_mut_slice()[start_idx..=end_idx].fill(f32::INFINITY);

                draw_scanline_textured_perspective(
                    &mut fb,
                    &mut zb,
                    &tex,
                    black_box(y),
                    black_box(x_start),
                    black_box(x_end),
                    black_box(start),
                    black_box(&gradients),
                );
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_scanline_lengths);
criterion_main!(benches);
