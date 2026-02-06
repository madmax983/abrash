use abrash::camera::Camera;
use abrash::math::Vec3;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_camera_view_projection(c: &mut Criterion) {
    let position = Vec3::new(0.0, 0.0, 5.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let fov = 1.0;
    let aspect = 1.33;
    let near = 0.1;
    let far = 100.0;

    c.bench_function("camera_view_projection", |b| {
        b.iter(|| {
            let camera = Camera::new(
                black_box(position),
                black_box(target),
                black_box(up),
                black_box(fov),
                black_box(aspect),
                black_box(near),
                black_box(far),
            );
            black_box(camera.view_projection_matrix());
        })
    });
}

criterion_group!(benches, bench_camera_view_projection);
criterion_main!(benches);
