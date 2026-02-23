use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::tile_renderer::{ClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn generate_small_triangles(count: usize) -> Vec<ClipTriangle> {
    let mut tris = Vec::with_capacity(count);
    for i in 0..count {
        let x = (i % 100) as f32 * 0.01 - 0.5;
        let y = (i / 100) as f32 * 0.01 - 0.5;
        let z = 5.0; // In front of camera
        let w = z; // Usually w=z for perspective projection before division?
        // No, ClipTriangle is ((Vec3, f32), ...). Usually input to rasterizer is in Clip Space.
        // If we simulate Clip Space, w is typically z.
        // Let's assume standard perspective where w = -z_view.
        // If we put them at z=5, w=5.

        // Small triangle size 0.001
        let size = 0.001;

        let v0 = (Vec3::new(x, y, z), w);
        let v1 = (Vec3::new(x + size, y, z), w);
        let v2 = (Vec3::new(x, y + size, z), w);

        let color = 0xFFFFFFFF;
        tris.push((v0, v1, v2, color));
    }
    tris
}

fn bench_prepare_triangle(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tr = TileRenderer::new(width, height);

    // 10,000 small triangles
    let triangles = generate_small_triangles(10_000);

    c.bench_function("prepare_10k_triangles", |b| {
        b.iter(|| {
            // We clear buffers to be fair, though rasterization is minimal
            zb.clear();
            // The main cost here should be prepare_triangle (10k calls)
            tr.render_batch(&mut fb, &mut zb, black_box(&triangles));
        });
    });
}

criterion_group!(benches, bench_prepare_triangle);
criterion_main!(benches);
