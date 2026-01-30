use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::primitives::{fill_triangle_3d, fill_triangle_textured};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_sample_nearest_1000(c: &mut Criterion) {
    c.bench_function("texture_sample_nearest_1000", |b| {
        let mut texture = Texture::new(64, 64);
        texture.set_pixel(32, 32, 0xFFFF0000); // Red center pixel

        b.iter(|| {
            let mut sum = 0u32;
            for i in 0..1000 {
                let u = (i as f32 * 0.123).rem_euclid(1.0);
                let v = (i as f32 * 0.456).rem_euclid(1.0);
                sum = sum.wrapping_add(black_box(texture.sample_nearest(u, v)));
            }
            black_box(sum);
        });
    });
}

fn bench_sample_bilinear_1000(c: &mut Criterion) {
    c.bench_function("texture_sample_bilinear_1000", |b| {
        let mut texture = Texture::new(64, 64);
        texture.set_pixel(32, 32, 0xFFFF0000);

        b.iter(|| {
            let mut sum = 0u32;
            for i in 0..1000 {
                let u = (i as f32 * 0.123).rem_euclid(1.0);
                let v = (i as f32 * 0.456).rem_euclid(1.0);
                sum = sum.wrapping_add(black_box(texture.sample_bilinear(u, v)));
            }
            black_box(sum);
        });
    });
}

fn bench_fill_triangle_textured_large(c: &mut Criterion) {
    c.bench_function("fill_triangle_textured_large", |b| {
        let mut fb = Framebuffer::new(800, 600);
        let mut zb = ZBuffer::new(800, 600);
        let texture = Texture::new(64, 64);

        // Large triangle covering significant screen area
        let v0 = ((Vec3::new(0.0, 2.0, -2.0), 1.0), Vec2::new(0.0, 0.0));
        let v1 = ((Vec3::new(-2.0, -2.0, -2.0), 1.0), Vec2::new(1.0, 0.0));
        let v2 = ((Vec3::new(2.0, -2.0, -2.0), 1.0), Vec2::new(0.5, 1.0));

        b.iter(|| {
            zb.clear();
            fill_triangle_textured(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(&texture),
            );
        });
    });
}

fn bench_fill_triangle_textured_small(c: &mut Criterion) {
    c.bench_function("fill_triangle_textured_small", |b| {
        let mut fb = Framebuffer::new(800, 600);
        let mut zb = ZBuffer::new(800, 600);
        let texture = Texture::new(64, 64);

        // Small triangle
        let v0 = ((Vec3::new(0.0, 0.1, -2.0), 1.0), Vec2::new(0.0, 0.0));
        let v1 = ((Vec3::new(-0.1, -0.1, -2.0), 1.0), Vec2::new(1.0, 0.0));
        let v2 = ((Vec3::new(0.1, -0.1, -2.0), 1.0), Vec2::new(0.5, 1.0));

        b.iter(|| {
            zb.clear();
            fill_triangle_textured(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(&texture),
            );
        });
    });
}

fn bench_triangle_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("triangle_fill_comparison");

    // Solid color triangle (baseline)
    group.bench_function("3d_solid", |b| {
        let mut fb = Framebuffer::new(800, 600);
        let mut zb = ZBuffer::new(800, 600);

        let v0 = (Vec3::new(0.0, 2.0, -2.0), 1.0);
        let v1 = (Vec3::new(-2.0, -2.0, -2.0), 1.0);
        let v2 = (Vec3::new(2.0, -2.0, -2.0), 1.0);

        b.iter(|| {
            zb.clear();
            fill_triangle_3d(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(0xFFFFFFFF),
            );
        });
    });

    // Textured triangle
    group.bench_function("3d_textured", |b| {
        let mut fb = Framebuffer::new(800, 600);
        let mut zb = ZBuffer::new(800, 600);
        let texture = Texture::new(64, 64);

        let v0 = ((Vec3::new(0.0, 2.0, -2.0), 1.0), Vec2::new(0.0, 0.0));
        let v1 = ((Vec3::new(-2.0, -2.0, -2.0), 1.0), Vec2::new(1.0, 0.0));
        let v2 = ((Vec3::new(2.0, -2.0, -2.0), 1.0), Vec2::new(0.5, 1.0));

        b.iter(|| {
            zb.clear();
            fill_triangle_textured(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(&texture),
            );
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_sample_nearest_1000,
    bench_sample_bilinear_1000,
    bench_fill_triangle_textured_large,
    bench_fill_triangle_textured_small,
    bench_triangle_comparison
);
criterion_main!(benches);
