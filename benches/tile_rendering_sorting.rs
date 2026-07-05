use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::{TexturedClipTriangle, TileRenderer};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rand::seq::SliceRandom;
use rand::thread_rng;

/// Generate N overlapping textured triangles at varying depths.
/// They are all centered roughly in the middle of the screen to ensure heavy overdraw.
fn generate_overlapping_triangles(count: usize) -> Vec<TexturedClipTriangle> {
    let mut tris = Vec::with_capacity(count);
    for i in 0..count {
        // Depth ranges from 2.0 to 10.0
        // We use 'i' to determine depth so we can easily sort later.
        // Smaller i = closer (smaller depth).
        let t = i as f32 / count as f32;
        let depth = 2.0 + t * 8.0;

        // Slight random offset to make it interesting, but keep them overlapping
        let offset_x = (i as f32 * 0.1).sin() * 0.1;
        let offset_y = (i as f32 * 0.1).cos() * 0.1;

        // Triangle covering center screen
        let v0 = (Vec3::new(offset_x, 0.5 + offset_y, depth), depth);
        let uv0 = Vec2::new(0.0, 0.0);

        let v1 = (Vec3::new(-0.5 + offset_x, -0.5 + offset_y, depth), depth);
        let uv1 = Vec2::new(0.0, 1.0);

        let v2 = (Vec3::new(0.5 + offset_x, -0.5 + offset_y, depth), depth);
        let uv2 = Vec2::new(1.0, 0.0);

        tris.push((v0, uv0, v1, uv1, v2, uv2));
    }
    tris
}

fn bench_sorting_impact(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let count = 500; // Enough to cause significant overdraw
    let raw_tris = generate_overlapping_triangles(count);

    // 1. Front-to-Back (Best case for Early-Z)
    // Smallest depth first. Our generate function creates small depth at small index.
    let mut tris_f2b = raw_tris.clone();
    // Already sorted by construction (depth = 2.0 + t*8.0), but let's be sure.
    tris_f2b.sort_by(|a, b| a.0.0.z.total_cmp(&b.0.0.z));

    // 2. Back-to-Front (Worst case / Painter's Algorithm)
    // Largest depth first.
    let mut tris_b2f = raw_tris.clone();
    tris_b2f.sort_by(|a, b| b.0.0.z.total_cmp(&a.0.0.z));

    // 3. Random Shuffle (Typical case)
    let mut tris_random = raw_tris;
    let mut rng = thread_rng();
    tris_random.shuffle(&mut rng);

    let texture = Texture::new(64, 64).unwrap();

    let mut group = c.benchmark_group("tile_rendering_sorting");

    group.bench_function("front_to_back", |b| {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);
        b.iter(|| {
            zb.clear();
            tr.render_batch_textured(&mut fb, &mut zb, black_box(&tris_f2b), &texture);
        });
    });

    group.bench_function("back_to_front", |b| {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);
        b.iter(|| {
            zb.clear();
            tr.render_batch_textured(&mut fb, &mut zb, black_box(&tris_b2f), &texture);
        });
    });

    group.bench_function("random", |b| {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);
        b.iter(|| {
            zb.clear();
            tr.render_batch_textured(&mut fb, &mut zb, black_box(&tris_random), &texture);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_sorting_impact);
criterion_main!(benches);
