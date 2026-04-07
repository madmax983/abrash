use abrash_core::math::{Vec2, Vec3};
use abrash_core::sdf;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_sdf_imprecise_flops(c: &mut Criterion) {
    let mut group = c.benchmark_group("sdf_imprecise_flops");

    // Test data for different functions
    let p_vec2 = Vec2::new(3.0, 4.0);
    let p_vec3 = Vec3::new(3.0, 4.0, 5.0);

    // rounded_star_2d
    group.bench_function("rounded_star_2d", |b| {
        b.iter(|| {
            sdf::rounded_star_2d(
                black_box(p_vec2),
                black_box(1.0),
                black_box(5),
                black_box(2.0),
            )
        })
    });

    // revolution_z
    group.bench_function("revolution_z", |b| {
        b.iter(|| {
            sdf::revolution_z(black_box(p_vec3), black_box(0.0), |q| {
                sdf::circle_2d(q, Vec2::new(2.0, 0.0), 0.3)
            })
        })
    });

    // mandelbrot_dist
    group.bench_function("mandelbrot_dist", |b| {
        b.iter(|| sdf::mandelbrot_dist(black_box(Vec2::new(0.5, 0.5)), black_box(64)))
    });

    // lens_2d
    group.bench_function("lens_2d", |b| {
        b.iter(|| sdf::lens_2d(black_box(p_vec2), black_box(0.5), black_box(1.0)))
    });

    // spiral_2d
    group.bench_function("spiral_2d", |b| {
        b.iter(|| sdf::spiral_2d(black_box(p_vec2), black_box(1.0), black_box(0.1)))
    });

    // polyline_2d
    let pts = [
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(2.0, 0.0),
    ];
    group.bench_function("polyline_2d", |b| {
        b.iter(|| sdf::polyline_2d(black_box(p_vec2), black_box(&pts)))
    });

    // disk_3d
    group.bench_function("disk_3d", |b| {
        b.iter(|| sdf::disk_3d(black_box(p_vec3), black_box(1.0), black_box(0.1)))
    });

    // diamond_3d
    group.bench_function("diamond_3d", |b| {
        b.iter(|| sdf::diamond_3d(black_box(p_vec3), black_box(1.0), black_box(0.5)))
    });

    // lemniscate_2d
    group.bench_function("lemniscate_2d", |b| {
        b.iter(|| sdf::lemniscate_2d(black_box(p_vec2), black_box(1.0)))
    });

    group.finish();
}

criterion_group!(benches, bench_sdf_imprecise_flops);
criterion_main!(benches);
