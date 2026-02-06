use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec3, Vec4};
use abrash::rasterizer::fill_triangle_gouraud;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_alpha_blending_opaque(c: &mut Criterion) {
    c.bench_function("alpha_blending_opaque", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();

        let v0 = (Vec3::new(0.0, 2.0, -2.0), 1.0);
        let v1 = (Vec3::new(-2.0, -2.0, -2.0), 1.0);
        let v2 = (Vec3::new(2.0, -2.0, -2.0), 1.0);

        let c_opaque = Vec4::new(0.0, 0.0, 1.0, 1.0);

        b.iter(|| {
            zb.clear();
            fill_triangle_gouraud(
                &mut fb,
                &mut zb,
                black_box(((v0.0, v0.1), c_opaque)),
                black_box(((v1.0, v1.1), c_opaque)),
                black_box(((v2.0, v2.1), c_opaque)),
            );
        });
    });
}

fn bench_alpha_blending_transparent(c: &mut Criterion) {
    c.bench_function("alpha_blending_transparent", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();

        fb.clear(0xFFFF0000);

        let v0 = (Vec3::new(0.0, 2.0, -2.0), 1.0);
        let v1 = (Vec3::new(-2.0, -2.0, -2.0), 1.0);
        let v2 = (Vec3::new(2.0, -2.0, -2.0), 1.0);

        let c_transparent = Vec4::new(0.0, 0.0, 1.0, 0.5);

        b.iter(|| {
            zb.clear();
            fill_triangle_gouraud(
                &mut fb,
                &mut zb,
                black_box(((v0.0, v0.1), c_transparent)),
                black_box(((v1.0, v1.1), c_transparent)),
                black_box(((v2.0, v2.1), c_transparent)),
            );
        });
    });
}

criterion_group!(benches, bench_alpha_blending_opaque, bench_alpha_blending_transparent);
criterion_main!(benches);
