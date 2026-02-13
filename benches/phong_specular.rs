use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_phong;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_phong_specular(c: &mut Criterion) {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let v0 = ((Vec3::new(0.0, 0.9, 1.0), 1.0), Vec3::new(0.0, 0.0, 1.0));
    let v1 = ((Vec3::new(-0.9, -0.9, 1.0), 1.0), Vec3::new(0.0, 0.0, 1.0));
    let v2 = ((Vec3::new(0.9, -0.9, 1.0), 1.0), Vec3::new(0.0, 0.0, 1.0));

    let light_dir = Vec3::new(0.0, 0.0, -1.0);
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let ambient = Vec3::new(0.1, 0.1, 0.1);
    let color = Vec3::new(1.0, 1.0, 1.0);
    let view_dir = Vec3::new(0.0, 0.0, 1.0);
    let specular_strength = 0.5;
    let shininess = 32.0;

    c.bench_function("phong_specular", |b| {
        b.iter(|| {
            fb.clear(0);
            zb.clear();
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
                black_box(view_dir),
                black_box(specular_strength),
                black_box(shininess),
            );
        })
    });
}

criterion_group!(benches, bench_phong_specular);
criterion_main!(benches);
