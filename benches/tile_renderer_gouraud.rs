use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_gouraud;
use abrash::tile_renderer::{ClipTriangleGouraud, TileRenderer};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn generate_gouraud_triangles(count: usize) -> Vec<ClipTriangleGouraud> {
    let mut tris = Vec::with_capacity(count);
    for i in 0..count {
        let t = i as f32 * 0.1;
        let offset_x = (t * 1.7).sin() * 0.15;
        let offset_y = (t * 2.3).cos() * 0.15;
        let depth = 3.0 + (t * 0.5).sin() * 2.0;

        let v0_pos = (Vec3::new(offset_x, 0.5 + offset_y, depth), depth);
        let v1_pos = (Vec3::new(-0.5 + offset_x, -0.5 + offset_y, depth), depth);
        let v2_pos = (Vec3::new(0.5 + offset_x, -0.5 + offset_y, depth), depth);

        let c0 = Vec3::new(1.0, 0.0, 0.0);
        let c1 = Vec3::new(0.0, 1.0, 0.0);
        let c2 = Vec3::new(0.0, 0.0, 1.0);

        tris.push((v0_pos, c0, v1_pos, c1, v2_pos, c2));
    }
    tris
}

fn bench_gouraud_tiled(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tr = TileRenderer::new(width, height);
    let tris = generate_gouraud_triangles(100);

    c.bench_function("fill_gouraud_tiled_1080p_100tris", |b| {
        b.iter(|| {
            zb.clear();
            tr.render_batch_gouraud(&mut fb, &mut zb, black_box(&tris));
        });
    });
}

fn bench_gouraud_scanline(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let tris = generate_gouraud_triangles(100);

    c.bench_function("fill_gouraud_scanline_1080p_100tris", |b| {
        b.iter(|| {
            zb.clear();
            for &(v0, c0, v1, c1, v2, c2) in black_box(&tris) {
                fill_triangle_gouraud(
                    &mut fb,
                    &mut zb,
                    (v0, c0),
                    (v1, c1),
                    (v2, c2),
                );
            }
        });
    });
}

criterion_group!(
    benches,
    bench_gouraud_tiled,
    bench_gouraud_scanline,
);
criterion_main!(benches);
