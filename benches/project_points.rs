use abrash::math::{ScreenPoint, Vec3, project_to_screen_optimized, project_triangle_to_screen};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

pub fn criterion_benchmark(c: &mut Criterion) {
    let half_width = 960.0;
    let half_height = 540.0;

    let v0 = Vec3::new(-0.5, 0.5, 5.0);
    let w0 = 5.0;
    let v1 = Vec3::new(0.5, 0.5, 5.0);
    let w1 = 5.0;
    let v2 = Vec3::new(0.0, -0.5, 5.0);
    let w2 = 5.0;

    c.bench_function("project_triangle_scalar", |b| {
        b.iter(|| {
            let p0 = project_to_screen_optimized(
                black_box(v0),
                black_box(w0),
                black_box(half_width),
                black_box(half_height),
            );
            let p1 = project_to_screen_optimized(
                black_box(v1),
                black_box(w1),
                black_box(half_width),
                black_box(half_height),
            );
            let p2 = project_to_screen_optimized(
                black_box(v2),
                black_box(w2),
                black_box(half_width),
                black_box(half_height),
            );
            (p0, p1, p2)
        })
    });

    c.bench_function("project_triangle_simd", |b| {
        b.iter(|| {
            project_triangle_to_screen(
                black_box(v0),
                black_box(w0),
                black_box(v1),
                black_box(w1),
                black_box(v2),
                black_box(w2),
                black_box(half_width),
                black_box(half_height),
            )
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
