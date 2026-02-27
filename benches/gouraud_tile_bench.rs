use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::{fill_triangle_gouraud, TileRenderer};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_gouraud_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("gouraud_rendering");

    // Scenario 1: 1080p, Low Poly (20 large triangles)
    // Scenario 2: 4K, Low Poly (20 large triangles)
    // Scenario 3: 4K, High Poly (500 small triangles) - Tile renderer might lose here due to binning overhead

    let v0 = (Vec3::new(-0.5, 0.5, 5.0), 1.0);
    let v1 = (Vec3::new(0.5, 0.5, 5.0), 1.0);
    let v2 = (Vec3::new(0.0, -0.5, 5.0), 1.0);
    let c0 = Vec3::new(1.0, 0.0, 0.0);
    let c1 = Vec3::new(0.0, 1.0, 0.0);
    let c2 = Vec3::new(0.0, 0.0, 1.0);

    // Create a batch of triangles
    let triangles = vec![((v0, c0), (v1, c1), (v2, c2)); 20];

    // Benchmark 1080p Immediate Mode (Baseline)
    group.bench_function("1080p_immediate_gouraud", |b| {
        let width = 1920;
        let height = 1080;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        b.iter(|| {
            fb.clear(0xFF000000);
            zb.clear();
            for tri in &triangles {
                fill_triangle_gouraud(&mut fb, &mut zb, tri.0, tri.1, tri.2);
            }
        });
    });

    // Benchmark 4K Immediate Mode (Baseline)
    group.bench_function("4k_immediate_gouraud", |b| {
        let width = 3840;
        let height = 2160;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        b.iter(|| {
            fb.clear(0xFF000000);
            zb.clear();
            for tri in &triangles {
                fill_triangle_gouraud(&mut fb, &mut zb, tri.0, tri.1, tri.2);
            }
        });
    });

    // Benchmark 4K Tile Mode (Target)
    // This will fail to compile until render_batch_gouraud is implemented
    /*
    group.bench_function("4k_tiled_gouraud", |b| {
        let width = 3840;
        let height = 2160;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut renderer = TileRenderer::new(width, height);

        b.iter(|| {
            fb.clear(0xFF000000);
            zb.clear();
            renderer.render_batch_gouraud(&mut fb, &mut zb, black_box(&triangles));
        });
    });
    */

    group.finish();
}

criterion_group!(benches, bench_gouraud_rendering);
criterion_main!(benches);
