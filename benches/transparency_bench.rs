use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_transparency(c: &mut Criterion) {
    let mut group = c.benchmark_group("transparency");
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Large triangle
    let v0 = (Vec3::new(0.0, 0.9, 5.0), 5.0);
    let v1 = (Vec3::new(-0.9, -0.9, 5.0), 5.0);
    let v2 = (Vec3::new(0.9, -0.9, 5.0), 5.0);

    // Opaque color
    group.bench_function("opaque", |b| {
        b.iter(|| {
            fb.clear(0xFF00_0000);
            zb.clear();
            fill_triangle_3d(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(0xFFFF_0000), // Opaque Red
            );
        });
    });

    // Transparent color
    group.bench_function("transparent", |b| {
        b.iter(|| {
            fb.clear(0xFF00_00FF); // Blue background
            zb.clear();
            fill_triangle_3d(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(0x80FF_0000), // 50% Red
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_transparency);
criterion_main!(benches);
