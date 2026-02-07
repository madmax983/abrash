use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::{Texture, fill_triangle_textured};
use abrash::tile_renderer::{TexturedClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn generate_textured_triangles(count: usize) -> Vec<TexturedClipTriangle> {
    let mut tris = Vec::with_capacity(count);
    for i in 0..count {
        let t = i as f32 * 0.1;
        let offset_x = (t * 1.7).sin() * 0.15;
        let offset_y = (t * 2.3).cos() * 0.15;
        let depth = 3.0 + (t * 0.5).sin() * 2.0; // Depth > 1.0 (inside frustum)

        // Triangle vertices in Clip Space (using W=depth for perspective)
        let v0_pos = (Vec3::new(offset_x * depth, (0.5 + offset_y) * depth, depth), depth);
        let v1_pos = (Vec3::new((-0.5 + offset_x) * depth, (-0.5 + offset_y) * depth, depth), depth);
        let v2_pos = (Vec3::new((0.5 + offset_x) * depth, (-0.5 + offset_y) * depth, depth), depth);

        // UVs
        let v0_uv = Vec2::new(0.5, 0.0);
        let v1_uv = Vec2::new(0.0, 1.0);
        let v2_uv = Vec2::new(1.0, 1.0);

        tris.push(((v0_pos, v0_uv), (v1_pos, v1_uv), (v2_pos, v2_uv)));
    }
    tris
}

fn bench_textured_tiled(c: &mut Criterion) {
    let texture = Texture::new(256, 256).unwrap();
    let width = 1920;
    let height = 1080;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tr = TileRenderer::new(width, height);

    let tris = generate_textured_triangles(100);

    c.bench_function("textured_tiled_1080p_100tris", |b| {
        b.iter(|| {
            // Note: TileRenderer clears internal buffers, but FB/ZB clearing is outside scope usually.
            // But here we want to measure full frame time including FB clear if that's part of the loop.
            // Usually we clear FB once per frame.
            fb.clear(0);
            zb.clear();
            tr.render_batch_textured(&mut fb, &mut zb, &texture, black_box(&tris));
        });
    });
}

fn bench_textured_scanline(c: &mut Criterion) {
    let texture = Texture::new(256, 256).unwrap();
    let width = 1920;
    let height = 1080;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let tris = generate_textured_triangles(100);

    c.bench_function("textured_scanline_1080p_100tris", |b| {
        b.iter(|| {
            fb.clear(0);
            zb.clear();
            for &(v0, v1, v2) in black_box(&tris) {
                fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &texture);
            }
        });
    });
}

criterion_group!(benches, bench_textured_tiled, bench_textured_scanline);
criterion_main!(benches);
