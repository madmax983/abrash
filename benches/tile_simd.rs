use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::tile_renderer::{ClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Generate large overlapping triangles that cover most of the screen.
/// This maximizes the time spent in `rasterize_scanline`, making SIMD improvements more visible.
fn generate_large_triangles(count: usize) -> Vec<ClipTriangle> {
    let mut tris = Vec::with_capacity(count);
    for i in 0..count {
        // Reverse depth so new triangles are always closer (forcing writes)
        // This stresses the write path of the rasterizer
        // Use values within 0.0-1.0 range for Z-buffer if possible, though clip space is usually -w to w.
        // project_to_screen divides by w.
        // We set w=1.0 so z_ndc = z.
        let depth = 0.9 - (i as f32 * 0.001);

        // Vertices that span the screen (roughly -1.0 to 1.0 in NDC)
        // Shift them slightly so they aren't identical
        let shift = (i as f32) * 0.01;

        let v0 = (Vec3::new(-1.0 + shift, 1.0 - shift, depth), 1.0);
        let v1 = (Vec3::new(-1.0 + shift, -1.0 + shift, depth), 1.0);
        let v2 = (Vec3::new(1.0 - shift, -1.0 + shift, depth), 1.0);

        let color = 0xFF00_00FF; // Red

        tris.push((v0, v1, v2, color));
    }
    tris
}

fn bench_tile_rasterization_4k(c: &mut Criterion) {
    // 4K resolution to ensure long scanlines and many tiles
    let width = 3840;
    let height = 2160;

    // 50 large triangles to provide substantial work
    let triangles = generate_large_triangles(50);

    let mut group = c.benchmark_group("tile_simd_4k");

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tr = TileRenderer::new(width, height);

    group.bench_function("render_batch_50_large_tris", |b| {
        b.iter(|| {
            zb.clear();
            // We use render_batch which includes binning, but for large triangles
            // and 4K resolution, rasterization time should dominate.
            tr.render_batch(&mut fb, &mut zb, black_box(&triangles));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_tile_rasterization_4k);
criterion_main!(benches);
