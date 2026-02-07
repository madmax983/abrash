use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::{Texture, fill_triangle_textured};
use abrash::tile_renderer::{TexturedClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main, BenchmarkId};

fn generate_textured_triangles(count: usize) -> Vec<TexturedClipTriangle> {
    let mut tris = Vec::with_capacity(count);
    for i in 0..count {
        let t = i as f32 * 0.1;
        let offset_x = (t * 1.7).sin() * 0.15;
        let offset_y = (t * 2.3).cos() * 0.15;
        let depth = 3.0 + (t * 0.5).sin() * 2.0;

        let v0 = (Vec3::new(offset_x, 0.5 + offset_y, depth), depth);
        let v1 = (Vec3::new(-0.5 + offset_x, -0.5 + offset_y, depth), depth);
        let v2 = (Vec3::new(0.5 + offset_x, -0.5 + offset_y, depth), depth);

        let uv0 = Vec2::new(0.0, 0.0);
        let uv1 = Vec2::new(1.0, 0.0);
        let uv2 = Vec2::new(0.5, 1.0);

        tris.push((v0, uv0, v1, uv1, v2, uv2));
    }
    tris
}

fn bench_textured_tiled(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let texture = Texture::checkered(256, 256, 0xFFFFFFFF, 0xFF000000).unwrap();
    let triangles = generate_textured_triangles(100);

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tr = TileRenderer::new(width, height);

    c.bench_function("textured_tiled_1080p_100tris", |b| {
        b.iter(|| {
            zb.clear();
            tr.render_batch_textured(&mut fb, &mut zb, black_box(&triangles), &texture);
        })
    });
}

fn bench_textured_scanline(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let texture = Texture::checkered(256, 256, 0xFFFFFFFF, 0xFF000000).unwrap();
    let triangles = generate_textured_triangles(100);

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    c.bench_function("textured_scanline_1080p_100tris", |b| {
        b.iter(|| {
            zb.clear();
            for &(v0, uv0, v1, uv1, v2, uv2) in black_box(&triangles) {
                fill_triangle_textured(&mut fb, &mut zb, (v0, uv0), (v1, uv1), (v2, uv2), &texture);
            }
        })
    });
}

criterion_group!(
    benches,
    bench_textured_tiled,
    bench_textured_scanline,
);
criterion_main!(benches);
