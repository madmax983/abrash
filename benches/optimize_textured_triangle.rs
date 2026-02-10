use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec2, Vec3};
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn bench_fill_triangle_textured(c: &mut Criterion) {
    let width = 1024;
    let height = 1024;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let texture = Texture::new(256, 256).unwrap();

    // Create a large triangle that covers a significant portion of the screen
    let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
    let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
    let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);

    let uv0 = Vec2::new(0.5, 0.0);
    let uv1 = Vec2::new(0.0, 1.0);
    let uv2 = Vec2::new(1.0, 1.0);

    c.bench_function("fill_triangle_textured_baseline", |b| {
        b.iter(|| {
            // We don't clear the buffers every time to stress the rasterization more than clearing
            // But realistically we should probably clear Z to ensure depth test passes?
            // Actually fill_triangle_textured does depth testing.
            // Let's reset Z buffer to allow overwriting.
            zb.clear();
            fill_triangle_textured(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box((v0, uv0)),
                black_box((v1, uv1)),
                black_box((v2, uv2)),
                black_box(&texture),
            );
        })
    });
}

criterion_group!(benches, bench_fill_triangle_textured);
criterion_main!(benches);
