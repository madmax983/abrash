use abrash::math::{Mat4, Vec3};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_transform_points_scalar(c: &mut Criterion) {
    c.bench_function("transform_points_scalar_1000", |b| {
        let m = Mat4::rotation_y(0.5);
        let points: Vec<Vec3> = (0..1000)
            .map(|i| Vec3::new(i as f32, i as f32, i as f32))
            .collect();
        let mut output = vec![(Vec3::default(), 0.0); 1000];

        b.iter(|| {
            m.transform_points(black_box(&points), black_box(&mut output));
        });
    });
}

fn bench_transform_points_manual_loop(c: &mut Criterion) {
    c.bench_function("transform_points_manual_loop_1000", |b| {
        let m = Mat4::rotation_y(0.5);
        let points: Vec<Vec3> = (0..1000)
            .map(|i| Vec3::new(i as f32, i as f32, i as f32))
            .collect();
        let mut output = vec![(Vec3::default(), 0.0); 1000];

        b.iter(|| {
            // Re-implement the loop manually to see if function call overhead matters
            let points_ref = black_box(&points);
            let output_ref = black_box(&mut output);
            for (i, p) in points_ref.iter().enumerate() {
                output_ref[i] = m.transform_point(*p);
            }
        });
    });
}

fn bench_transform_points_scalar_100k(c: &mut Criterion) {
    c.bench_function("transform_points_scalar_100k", |b| {
        let m = Mat4::rotation_y(0.5);
        let points: Vec<Vec3> = (0..100_000)
            .map(|i| Vec3::new(i as f32, i as f32, i as f32))
            .collect();
        let mut output = vec![(Vec3::default(), 0.0); 100_000];

        b.iter(|| {
            m.transform_points(black_box(&points), black_box(&mut output));
        });
    });
}

criterion_group!(
    benches,
    bench_transform_points_scalar,
    bench_transform_points_manual_loop,
    bench_transform_points_scalar_100k
);
criterion_main!(benches);
