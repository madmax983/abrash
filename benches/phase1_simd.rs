use abrash::{
    framebuffer::Framebuffer,
    hiz_buffer::HiZBuffer,
    math::Vec3,
    rasterizer::{ClipTriangle, TileRenderer},
    zbuffer::ZBuffer,
};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Generate test triangles for benchmarking
fn generate_test_scene(count: usize, width: u32, height: u32) -> Vec<ClipTriangle> {
    let mut triangles = Vec::with_capacity(count);

    for i in 0..count {
        let x = ((i % 20) as f32) * (width as f32 / 20.0);
        let y = ((i / 20) as f32) * (height as f32 / 20.0);
        let depth = 5.0 + ((i % 5) as f32) * 2.0; // 5 depth layers
        let size = 100.0;

        let v0 = (Vec3::new(x, y, depth), 1.0);
        let v1 = (Vec3::new(x + size, y, depth + 0.1), 1.0);
        let v2 = (Vec3::new(x + size / 2.0, y + size, depth + 0.2), 1.0);

        let color = 0xFF_00_00_00 | ((i as u32) << 8);
        triangles.push((v0, v1, v2, color));
    }

    triangles
}

/// Scene 1: Sparse (10 triangles at 4K)
fn bench_scene1_sparse_4k(c: &mut Criterion) {
    let width = 3840;
    let height = 2160;
    let triangles = generate_test_scene(10, width, height);

    let mut group = c.benchmark_group("scene1_sparse_4k");

    group.bench_function("baseline", |b| {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut renderer = TileRenderer::new(width, height);

        b.iter(|| {
            fb.clear(black_box(0xFF_00_00_00));
            zb.clear();
            renderer.render_batch(&mut fb, &mut zb, black_box(&triangles));
        });
    });

    group.finish();
}

/// Scene 2: Medium (100 triangles at 1080p)
fn bench_scene2_medium_1080p(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let triangles = generate_test_scene(100, width, height);

    let mut group = c.benchmark_group("scene2_medium_1080p");

    group.bench_function("baseline", |b| {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut renderer = TileRenderer::new(width, height);

        b.iter(|| {
            fb.clear(black_box(0xFF_00_00_00));
            zb.clear();
            renderer.render_batch(&mut fb, &mut zb, black_box(&triangles));
        });
    });

    #[cfg(feature = "simd")]
    group.bench_function("with_hiz", |b| {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut renderer = TileRenderer::new(width, height);
        renderer.enable_hiz();

        b.iter(|| {
            fb.clear(black_box(0xFF_00_00_00));
            zb.clear();
            renderer.render_batch(&mut fb, &mut zb, black_box(&triangles));
        });
    });

    group.finish();
}

/// Scene 3: Dense (1000 triangles at 4K)
fn bench_scene3_dense_4k(c: &mut Criterion) {
    let width = 3840;
    let height = 2160;
    let triangles = generate_test_scene(1000, width, height);

    let mut group = c.benchmark_group("scene3_dense_4k");
    group.sample_size(10); // Fewer samples for slow benchmark

    group.bench_function("baseline", |b| {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut renderer = TileRenderer::new(width, height);

        b.iter(|| {
            fb.clear(black_box(0xFF_00_00_00));
            zb.clear();
            renderer.render_batch(&mut fb, &mut zb, black_box(&triangles));
        });
    });

    #[cfg(feature = "simd")]
    group.bench_function("with_hiz", |b| {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut renderer = TileRenderer::new(width, height);
        renderer.enable_hiz();

        b.iter(|| {
            fb.clear(black_box(0xFF_00_00_00));
            zb.clear();
            renderer.render_batch(&mut fb, &mut zb, black_box(&triangles));
        });
    });

    group.finish();
}

/// Benchmark Hi-Z pyramid build time
fn bench_hiz_pyramid_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("hiz_pyramid_build");

    // 1080p
    group.bench_function("1080p", |b| {
        let zb = ZBuffer::new(1920, 1080).unwrap();
        let mut hiz = HiZBuffer::new(1920, 1080);

        b.iter(|| {
            hiz.build_pyramid(black_box(&zb));
        });
    });

    // 4K
    group.bench_function("4k", |b| {
        let zb = ZBuffer::new(3840, 2160).unwrap();
        let mut hiz = HiZBuffer::new(3840, 2160);

        b.iter(|| {
            hiz.build_pyramid(black_box(&zb));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_scene1_sparse_4k,
    bench_scene2_medium_1080p,
    bench_scene3_dense_4k,
    bench_hiz_pyramid_build
);
criterion_main!(benches);
