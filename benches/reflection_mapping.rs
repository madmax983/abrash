use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_reflection(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let mut tex = Texture::new(256, 256).unwrap();
    // Access pixels using set_pixel to avoid private field access issues
    for y in 0..256 {
        for x in 0..256 {
            tex.set_pixel(x, y, 0xFFFFFFFF);
        }
    }

    // Reflection typically involves transforming coordinates or texture lookup.
    // This is a placeholder for a reflection mapping benchmark.
    // Assuming fill_triangle_textured is used for rendering the reflective surface
    // with a pre-computed environment map.

    let v0 = ((Vec3::new(0.0, 0.5, 5.0), 1.0), Vec2::new(0.5, 0.0));
    let v1 = ((Vec3::new(-0.5, -0.5, 5.0), 1.0), Vec2::new(0.0, 1.0));
    let v2 = ((Vec3::new(0.5, -0.5, 5.0), 1.0), Vec2::new(1.0, 1.0));

    c.bench_function("reflection_mapping_textured", |b| {
        b.iter(|| {
            zb.clear();
            fill_triangle_textured(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(&tex),
            );
        });
    });
}

criterion_group!(benches, bench_reflection);
criterion_main!(benches);
