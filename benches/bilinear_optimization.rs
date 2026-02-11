use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::{FilterMode, Texture};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_bilinear_magnification(c: &mut Criterion) {
    let mut group = c.benchmark_group("bilinear_optimization");

    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Small texture (16x16) stretched over a large triangle (magnification)
    // This maximizes the cache hit rate in the bilinear rasterizer.
    let mut tex = Texture::checkered(16, 16, 0xFFFFFFFF, 0xFF000000).unwrap();
    tex.filter_mode = FilterMode::Bilinear;

    let v0 = ((Vec3::new(0.0, 0.9, 0.5), 1.0), Vec2::new(0.5, 0.0));
    let v1 = ((Vec3::new(-0.9, -0.9, 0.5), 1.0), Vec2::new(0.0, 1.0));
    let v2 = ((Vec3::new(0.9, -0.9, 0.5), 1.0), Vec2::new(1.0, 1.0));

    group.bench_function("magnification_cache_hit", |b| {
        b.iter(|| {
            zb.clear();
            fill_triangle_textured(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                &tex,
            );
        });
    });

    // Large texture (1024x1024) mapped 1:1 or minified
    // This minimizes cache hits.
    let mut tex_large = Texture::checkered(1024, 1024, 0xFFFFFFFF, 0xFF000000).unwrap();
    tex_large.filter_mode = FilterMode::Bilinear;

    group.bench_function("minification_cache_miss", |b| {
        b.iter(|| {
            zb.clear();
            fill_triangle_textured(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                &tex_large,
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_bilinear_magnification);
criterion_main!(benches);
