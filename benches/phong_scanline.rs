use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_phong;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_phong_scanline_performance(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Full screen quad composed of two triangles
    // This maximizes pixel coverage (scanline work) relative to vertex setup.
    let v0 = ((Vec3::new(-1.0, 1.0, 1.0), 1.0), Vec3::new(0.0, 0.0, 1.0));
    let v1 = ((Vec3::new(-1.0, -1.0, 1.0), 1.0), Vec3::new(0.0, 0.0, 1.0));
    let v2 = ((Vec3::new(1.0, -1.0, 1.0), 1.0), Vec3::new(0.0, 0.0, 1.0));
    let v3 = ((Vec3::new(1.0, 1.0, 1.0), 1.0), Vec3::new(0.0, 0.0, 1.0));

    let light_dir = Vec3::new(0.0, 0.0, -1.0);
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let ambient = Vec3::new(0.1, 0.1, 0.1);
    let color = Vec3::new(1.0, 1.0, 1.0);

    c.bench_function("phong_scanline_heavy", |b| {
        b.iter(|| {
            fb.clear(0);
            zb.clear();
            // Triangle 1
            fill_triangle_phong(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(color),
                black_box(light_dir),
                black_box(light_color),
                black_box(ambient),
            );
            // Triangle 2
             fill_triangle_phong(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(v0),
                black_box(v2),
                black_box(v3),
                black_box(color),
                black_box(light_dir),
                black_box(light_color),
                black_box(ambient),
            );
        })
    });
}

criterion_group!(benches, bench_phong_scanline_performance);
criterion_main!(benches);
