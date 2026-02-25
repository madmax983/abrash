use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::{ClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Generate overlapping triangles with random depths to simulate overdraw.
/// We want a scenario where many triangles cover the same pixels.
fn generate_overdraw_scene(count: usize) -> Vec<ClipTriangle> {
    let mut tris = Vec::with_capacity(count);

    // Use a deterministic "random" sequence
    let mut seed = 12345u64;
    let mut rand = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as f32 / 4294967296.0
    };

    for _ in 0..count {
        // Random depth between 2.0 and 10.0
        let depth = 2.0 + rand() * 8.0;

        // Random position, but centered around screen center
        // Screen coords in clip space are -1 to 1.
        // We want large triangles covering the center.
        let center_x = (rand() - 0.5) * 0.5; // -0.25 to 0.25
        let center_y = (rand() - 0.5) * 0.5; // -0.25 to 0.25

        // Size
        let size = 0.5 + rand() * 0.5; // 0.5 to 1.0

        let v0 = (Vec3::new(center_x, center_y + size, depth), depth);
        let v1 = (Vec3::new(center_x - size, center_y - size, depth), depth);
        let v2 = (Vec3::new(center_x + size, center_y - size, depth), depth);

        // Random color
        let r = (rand() * 255.0) as u32;
        let g = (rand() * 255.0) as u32;
        let b = (rand() * 255.0) as u32;
        let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

        tris.push((v0, v1, v2, color));
    }

    // We intentionally do NOT sort them here, so they are in random order.
    tris
}

fn bench_overdraw_1080p(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let triangle_count = 500;

    let tris = generate_overdraw_scene(triangle_count);

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tr = TileRenderer::new(width, height);

    let mut group = c.benchmark_group("overdraw");
    group.sample_size(20); // Reducing sample size as it might be slow

    group.bench_function("tiled_500_tris", |b| {
        b.iter(|| {
            // We clear ZBuffer manually to simulate frame start
            zb.clear();
            // Framebuffer clear is done inside TileRenderer for tiles, but usually we clear whole FB.
            // TileRenderer clears tiles it touches.

            tr.render_batch(&mut fb, &mut zb, black_box(&tris));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_overdraw_1080p);
criterion_main!(benches);
