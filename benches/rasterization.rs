use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::{fill_triangle_3d, fill_triangle_gouraud, fill_triangle_textured};
use abrash::texture::{FilterMode, Texture};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

use abrash::math::Vec2;

fn bench_fill_triangle_gouraud(c: &mut Criterion) {
    c.bench_function("fill_triangle_gouraud", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();

        // Use visible coordinates inside [-w, w]
        let v0 = (Vec3::new(0.0, 0.9, 0.5), 1.0);
        let v1 = (Vec3::new(-0.9, -0.9, 0.5), 1.0);
        let v2 = (Vec3::new(0.9, -0.9, 0.5), 1.0);

        let c0 = Vec3::new(1.0, 0.0, 0.0);
        let c1 = Vec3::new(0.0, 1.0, 0.0);
        let c2 = Vec3::new(0.0, 0.0, 1.0);

        b.iter(|| {
            zb.clear();
            fill_triangle_gouraud(
                &mut fb,
                &mut zb,
                black_box((v0, c0)),
                black_box((v1, c1)),
                black_box((v2, c2)),
            );
        });
    });
}

fn bench_fill_triangle_3d_large(c: &mut Criterion) {
    c.bench_function("fill_triangle_3d_large", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();

        // A large triangle covering a significant portion of the screen
        // Visible in [-1, 1] range
        let v0 = (Vec3::new(0.0, 0.9, 0.5), 1.0);
        let v1 = (Vec3::new(-0.9, -0.9, 0.5), 1.0);
        let v2 = (Vec3::new(0.9, -0.9, 0.5), 1.0);

        b.iter(|| {
            // Clear buffers to ensure consistent state
            zb.clear();
            fill_triangle_3d(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(0xFFFF_FFFF),
            );
        });
    });
}

fn bench_fill_triangle_3d_small(c: &mut Criterion) {
    c.bench_function("fill_triangle_3d_small", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();

        // Small triangle
        let v0 = (Vec3::new(0.0, 0.1, 0.5), 1.0);
        let v1 = (Vec3::new(-0.1, -0.1, 0.5), 1.0);
        let v2 = (Vec3::new(0.1, -0.1, 0.5), 1.0);

        b.iter(|| {
            zb.clear();
            fill_triangle_3d(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(0xFFFF_FFFF),
            );
        });
    });
}

fn bench_fill_triangle_textured(c: &mut Criterion) {
    c.bench_function("fill_triangle_textured", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();
        let tex = Texture::checkered(256, 256, 0xFFFF_FFFF, 0xFF00_0000).unwrap();

        let v0 = ((Vec3::new(0.0, 0.9, 0.5), 1.0), Vec2::new(0.5, 0.0));
        let v1 = ((Vec3::new(-0.9, -0.9, 0.5), 1.0), Vec2::new(0.0, 1.0));
        let v2 = ((Vec3::new(0.9, -0.9, 0.5), 1.0), Vec2::new(1.0, 1.0));

        b.iter(|| {
            zb.clear();
            fill_triangle_textured(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                &tex,
            );
        });
    });
}

fn bench_fill_triangle_textured_perspective_stress(c: &mut Criterion) {
    c.bench_function("fill_triangle_textured_perspective_stress", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();
        let tex = Texture::checkered(256, 256, 0xFFFF_FFFF, 0xFF00_0000).unwrap();

        // High perspective distortion: v1 and v2 have larger W (farther away)
        let v0 = ((Vec3::new(0.0, 0.9, 0.5), 1.0), Vec2::new(0.5, 0.0));
        // v1, v2 at w=4.0. To map to same screen space as previous, multiply x,y,z by 4?
        // Let's keep them somewhat visible.
        // If w=4, range is [-4, 4].
        let v1 = ((Vec3::new(-3.0, -3.0, 2.0), 4.0), Vec2::new(0.0, 1.0));
        let v2 = ((Vec3::new(3.0, -3.0, 2.0), 4.0), Vec2::new(1.0, 1.0));

        b.iter(|| {
            zb.clear();
            fill_triangle_textured(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                &tex,
            );
        });
    });
}

fn bench_fill_triangle_textured_bilinear(c: &mut Criterion) {
    c.bench_function("fill_triangle_textured_bilinear", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();
        let mut tex = Texture::checkered(256, 256, 0xFFFF_FFFF, 0xFF00_0000).unwrap();
        tex.filter_mode = FilterMode::Bilinear;

        let v0 = ((Vec3::new(0.0, 0.9, 0.5), 1.0), Vec2::new(0.5, 0.0));
        let v1 = ((Vec3::new(-0.9, -0.9, 0.5), 1.0), Vec2::new(0.0, 1.0));
        let v2 = ((Vec3::new(0.9, -0.9, 0.5), 1.0), Vec2::new(1.0, 1.0));

        b.iter(|| {
            zb.clear();
            fill_triangle_textured(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                &tex,
            );
        });
    });
}

fn bench_fill_triangle_clipped(c: &mut Criterion) {
    c.bench_function("fill_triangle_clipped", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();

        // Triangle straddling the near plane (w=0).
        // v0 is behind (w = -1.0), v1/v2 in front.
        let v0 = (Vec3::new(0.0, 1.0, 0.0), -1.0);
        let v1 = (Vec3::new(0.5, -0.5, 0.0), 1.0);
        let v2 = (Vec3::new(-0.5, -0.5, 0.0), 1.0);

        b.iter(|| {
            zb.clear();
            fill_triangle_3d(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(0xFFFF_FFFF),
            );
        });
    });
}

fn bench_fill_triangle_textured_small_batch(c: &mut Criterion) {
    c.bench_function("fill_triangle_textured_small_batch", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();
        let tex = Texture::checkered(256, 256, 0xFFFF_FFFF, 0xFF00_0000).unwrap();

        // 100 small triangles
        let triangles: Vec<_> = (0..100).map(|i| {
            let offset = (i as f32) * 5.0;
            // Ensure they are on screen
            let x = (offset % 700.0) + 10.0;
            let y = (offset % 500.0) + 10.0;
            let v0 = ((Vec3::new(x, y, 0.5), 1.0), Vec2::new(0.0, 0.0));
            let v1 = ((Vec3::new(x + 10.0, y, 0.5), 1.0), Vec2::new(1.0, 0.0));
            let v2 = ((Vec3::new(x, y + 10.0, 0.5), 1.0), Vec2::new(0.0, 1.0));
            (v0, v1, v2)
        }).collect();

        b.iter(|| {
            zb.clear();
            for (v0, v1, v2) in &triangles {
                fill_triangle_textured(
                    &mut fb,
                    &mut zb,
                    black_box(*v0),
                    black_box(*v1),
                    black_box(*v2),
                    &tex,
                );
            }
        });
    });
}

criterion_group!(
    benches,
    bench_fill_triangle_3d_large,
    bench_fill_triangle_3d_small,
    bench_fill_triangle_gouraud,
    bench_fill_triangle_textured,
    bench_fill_triangle_textured_perspective_stress,
    bench_fill_triangle_textured_bilinear,
    bench_fill_triangle_clipped,
    bench_fill_triangle_textured_small_batch,
    bench_get_pixel_bilinear_fixed
);
criterion_main!(benches);

fn bench_get_pixel_bilinear_fixed(c: &mut Criterion) {
    c.bench_function("get_pixel_bilinear_fixed", |b| {
        let tex = Texture::checkered(256, 256, 0xFFFF_FFFF, 0xFF00_0000).unwrap();
        // Simulate iterating over a span
        let mut u = 100 * 256;
        let mut v = 100 * 256;
        let du = 100;
        let dv = 50;
        let mask = 0xFFFF; // Keep within 0..65535 (256.0 in 24.8)

        b.iter(|| {
            // Unroll slightly
            black_box(tex.get_pixel_bilinear_fixed(u, v));
            u = (u + du) & mask;
            v = (v + dv) & mask;
            black_box(tex.get_pixel_bilinear_fixed(u, v));
            u = (u + du) & mask;
            v = (v + dv) & mask;
            black_box(tex.get_pixel_bilinear_fixed(u, v));
            u = (u + du) & mask;
            v = (v + dv) & mask;
            black_box(tex.get_pixel_bilinear_fixed(u, v));
            u = (u + du) & mask;
            v = (v + dv) & mask;
        });
    });
}
