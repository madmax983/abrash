use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::{fill_triangle_3d, fill_triangle_textured};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_textured_transparency_opaque(c: &mut Criterion) {
    c.bench_function("textured_transparency_opaque", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();
        // Opaque white
        let tex = Texture::checkered(256, 256, 0xFFFF_FFFF, 0xFFFF_FFFF).unwrap();

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

fn bench_textured_transparency_transparent(c: &mut Criterion) {
    c.bench_function("textured_transparency_transparent", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();
        // Fully transparent
        let mut tex = Texture::new(256, 256).unwrap();
        // Access pixels using set_pixel to avoid private field access issues
        for y in 0..256 {
            for x in 0..256 {
                tex.set_pixel(x, y, 0x0000_0000);
            }
        }

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

fn bench_textured_transparency_blended(c: &mut Criterion) {
    c.bench_function("textured_transparency_blended", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();
        // 50% Alpha (0x80)
        let mut tex = Texture::new(256, 256).unwrap();
        // Access pixels using set_pixel to avoid private field access issues
        for y in 0..256 {
            for x in 0..256 {
                tex.set_pixel(x, y, 0x80FF_FFFF);
            }
        }

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

fn bench_textured_transparency_mixed(c: &mut Criterion) {
    c.bench_function("textured_transparency_mixed", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();
        // Checkerboard: Opaque (FF) and Transparent (00)
        // This tests branch prediction stress.
        let tex = Texture::checkered(256, 256, 0xFFFF_FFFF, 0x0000_0000).unwrap();

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

fn bench_flat_transparency_opaque(c: &mut Criterion) {
    c.bench_function("flat_transparency_opaque", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();

        let v0 = (Vec3::new(0.0, 0.9, 0.5), 1.0);
        let v1 = (Vec3::new(-0.9, -0.9, 0.5), 1.0);
        let v2 = (Vec3::new(0.9, -0.9, 0.5), 1.0);
        let color = 0xFFFF_FFFF; // Opaque White

        b.iter(|| {
            zb.clear();
            fill_triangle_3d(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(color),
            );
        });
    });
}

fn bench_flat_transparency_transparent(c: &mut Criterion) {
    c.bench_function("flat_transparency_transparent", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();

        let v0 = (Vec3::new(0.0, 0.9, 0.5), 1.0);
        let v1 = (Vec3::new(-0.9, -0.9, 0.5), 1.0);
        let v2 = (Vec3::new(0.9, -0.9, 0.5), 1.0);
        let color = 0x00FF_FFFF; // Fully Transparent White

        b.iter(|| {
            zb.clear();
            fill_triangle_3d(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(color),
            );
        });
    });
}

fn bench_flat_transparency_blended(c: &mut Criterion) {
    c.bench_function("flat_transparency_blended", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();

        let v0 = (Vec3::new(0.0, 0.9, 0.5), 1.0);
        let v1 = (Vec3::new(-0.9, -0.9, 0.5), 1.0);
        let v2 = (Vec3::new(0.9, -0.9, 0.5), 1.0);
        let color = 0x80FF_FFFF; // 50% Alpha

        b.iter(|| {
            zb.clear();
            fill_triangle_3d(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(color),
            );
        });
    });
}

criterion_group!(
    benches,
    bench_textured_transparency_opaque,
    bench_textured_transparency_transparent,
    bench_textured_transparency_blended,
    bench_textured_transparency_mixed,
    bench_flat_transparency_opaque,
    bench_flat_transparency_transparent,
    bench_flat_transparency_blended,
);
criterion_main!(benches);
