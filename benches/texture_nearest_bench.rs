use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::{FilterMode, Texture};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_texture_nearest(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Create a large texture to ensure we hit memory
    let tex_size = 1024;
    let mut texture = Texture::new(tex_size, tex_size).unwrap();
    // Fill texture to force page faults / cache population
    for y in 0..tex_size {
        for x in 0..tex_size {
            texture.set_pixel(x, y, 0xFFFFFFFF);
        }
    }
    texture.filter_mode = FilterMode::Nearest;

    // Large triangle covering most of the screen
    let v0 = ((Vec3::new(-5.0, 5.0, 5.0), 5.0), Vec2::new(0.0, 0.0));
    let v1 = ((Vec3::new(5.0, 5.0, 5.0), 5.0), Vec2::new(1.0, 0.0));
    let v2 = ((Vec3::new(0.0, -5.0, 5.0), 5.0), Vec2::new(0.5, 1.0));

    c.bench_function("fill_triangle_textured_nearest_large", |b| {
        b.iter(|| {
            // Clear buffers efficiently (optional, but realistic)
            fb.clear(0);
            zb.clear();

            fill_triangle_textured(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(&texture),
            );
        });
    });
}

criterion_group!(benches, bench_texture_nearest);
criterion_main!(benches);
