use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::pipeline::{Vertex, fill_triangle_3d, fill_triangle_gouraud};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_fill_triangle_gouraud(c: &mut Criterion) {
    c.bench_function("fill_triangle_gouraud", |b| {
        let mut fb = Framebuffer::new(800, 600);
        let mut zb = ZBuffer::new(800, 600);

        let v0 = Vertex {
            position: Vec3::new(0.0, 2.0, -2.0),
            w: 1.0,
            color: Vec3::new(1.0, 0.0, 0.0),
        };
        let v1 = Vertex {
            position: Vec3::new(-2.0, -2.0, -2.0),
            w: 1.0,
            color: Vec3::new(0.0, 1.0, 0.0),
        };
        let v2 = Vertex {
            position: Vec3::new(2.0, -2.0, -2.0),
            w: 1.0,
            color: Vec3::new(0.0, 0.0, 1.0),
        };

        b.iter(|| {
            zb.clear();
            fill_triangle_gouraud(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
            );
        });
    });
}

fn bench_fill_triangle_3d_large(c: &mut Criterion) {
    c.bench_function("fill_triangle_3d_large", |b| {
        let mut fb = Framebuffer::new(800, 600);
        let mut zb = ZBuffer::new(800, 600);

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
                black_box(0xFFFFFFFF),
            );
        });
    });
}

fn bench_fill_triangle_3d_small(c: &mut Criterion) {
    c.bench_function("fill_triangle_3d_small", |b| {
        let mut fb = Framebuffer::new(800, 600);
        let mut zb = ZBuffer::new(800, 600);

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
                black_box(0xFFFFFFFF),
            );
        });
    });
}

criterion_group!(
    benches,
    bench_fill_triangle_3d_large,
    bench_fill_triangle_3d_small,
    bench_fill_triangle_gouraud
);
criterion_main!(benches);
