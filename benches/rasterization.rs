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

        let v0 = (Vec3::new(0.0, 2.0, -2.0), 1.0);
        let v1 = (Vec3::new(-2.0, -2.0, -2.0), 1.0);
        let v2 = (Vec3::new(2.0, -2.0, -2.0), 1.0);

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
        let v0 = (Vec3::new(0.0, 2.0, -2.0), 1.0);
        let v1 = (Vec3::new(-2.0, -2.0, -2.0), 1.0);
        let v2 = (Vec3::new(2.0, -2.0, -2.0), 1.0);

        b.iter(|| {
            // Clear buffers to ensure consistent state (though fill_triangle overwrites usually)
            // But for benchmarking rasterization speed, we might want to just draw over.
            // However, ZBuffer state affects early outs?
            // The current implementation writes if new depth < old depth.
            // If we don't clear, after first iter, zbuffer is full.
            // So we must clear zbuffer.
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

        let v0 = (Vec3::new(0.0, 0.1, -2.0), 1.0);
        let v1 = (Vec3::new(-0.1, -0.1, -2.0), 1.0);
        let v2 = (Vec3::new(0.1, -0.1, -2.0), 1.0);

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

        let v0 = ((Vec3::new(0.0, 2.0, -2.0), 1.0), Vec2::new(0.5, 0.0));
        let v1 = ((Vec3::new(-2.0, -2.0, -2.0), 1.0), Vec2::new(0.0, 1.0));
        let v2 = ((Vec3::new(2.0, -2.0, -2.0), 1.0), Vec2::new(1.0, 1.0));

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

        // High perspective distortion
        let v0 = ((Vec3::new(0.0, 2.0, -2.0), 1.0), Vec2::new(0.5, 0.0));
        let v1 = ((Vec3::new(-2.0, -2.0, -5.0), 4.0), Vec2::new(0.0, 1.0));
        let v2 = ((Vec3::new(2.0, -2.0, -5.0), 4.0), Vec2::new(1.0, 1.0));

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

        let v0 = ((Vec3::new(0.0, 2.0, -2.0), 1.0), Vec2::new(0.5, 0.0));
        let v1 = ((Vec3::new(-2.0, -2.0, -2.0), 1.0), Vec2::new(0.0, 1.0));
        let v2 = ((Vec3::new(2.0, -2.0, -2.0), 1.0), Vec2::new(1.0, 1.0));

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

criterion_group!(
    benches,
    bench_fill_triangle_3d_large,
    bench_fill_triangle_3d_small,
    bench_fill_triangle_gouraud,
    bench_fill_triangle_textured,
    bench_fill_triangle_textured_perspective_stress,
    bench_fill_triangle_textured_bilinear,
    bench_fill_triangle_clipped,
);
criterion_main!(benches);
