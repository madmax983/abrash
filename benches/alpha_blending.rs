use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec3, Vec4};
use abrash::rasterizer::fill_triangle_gouraud;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_fill_triangle_gouraud_opaque(c: &mut Criterion) {
    c.bench_function("fill_triangle_gouraud_opaque", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();

        // Large triangle
        let v0 = (Vec3::new(0.0, 2.0, -2.0), 1.0);
        let v1 = (Vec3::new(-2.0, -2.0, -2.0), 1.0);
        let v2 = (Vec3::new(2.0, -2.0, -2.0), 1.0);

        let c0 = Vec4::new(1.0, 0.0, 0.0, 1.0);
        let c1 = Vec4::new(0.0, 1.0, 0.0, 1.0);
        let c2 = Vec4::new(0.0, 0.0, 1.0, 1.0);

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

fn bench_fill_triangle_gouraud_transparent(c: &mut Criterion) {
    c.bench_function("fill_triangle_gouraud_transparent", |b| {
        let mut fb = Framebuffer::new(800, 600).unwrap();
        let mut zb = ZBuffer::new(800, 600).unwrap();

        // Large triangle
        let v0 = (Vec3::new(0.0, 2.0, -2.0), 1.0);
        let v1 = (Vec3::new(-2.0, -2.0, -2.0), 1.0);
        let v2 = (Vec3::new(2.0, -2.0, -2.0), 1.0);

        // Alpha 0.5
        let c0 = Vec4::new(1.0, 0.0, 0.0, 0.5);
        let c1 = Vec4::new(0.0, 1.0, 0.0, 0.5);
        let c2 = Vec4::new(0.0, 0.0, 1.0, 0.5);

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

criterion_group!(
    benches,
    bench_fill_triangle_gouraud_opaque,
    bench_fill_triangle_gouraud_transparent
);
criterion_main!(benches);
