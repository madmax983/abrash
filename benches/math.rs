use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash::math::{Vec2, Mat2};

fn bench_vec2_add(c: &mut Criterion) {
    c.bench_function("vec2_add", |b| {
        let a = Vec2::new(1.0, 2.0);
        let v = Vec2::new(3.0, 4.0);
        b.iter(|| black_box(a + v));
    });
}

fn bench_vec2_mul(c: &mut Criterion) {
    c.bench_function("vec2_mul", |b| {
        let v = Vec2::new(3.0, 4.0);
        b.iter(|| black_box(v * 2.5));
    });
}

fn bench_mat2_transform(c: &mut Criterion) {
    c.bench_function("mat2_transform", |b| {
        let mat = Mat2::rotation(0.785); // 45 degrees
        let v = Vec2::new(3.0, 4.0);
        b.iter(|| black_box(mat.transform(v)));
    });
}

fn bench_mat2_batch_transform(c: &mut Criterion) {
    c.bench_function("mat2_batch_transform_100", |b| {
        let mat = Mat2::rotation(0.785);
        let vertices: Vec<Vec2> = (0..100)
            .map(|i| Vec2::new(i as f32, i as f32))
            .collect();

        b.iter(|| {
            vertices.iter()
                .map(|&v| mat.transform(v))
                .collect::<Vec<_>>()
        });
    });
}

fn bench_mat2_transform_in_place(c: &mut Criterion) {
    c.bench_function("mat2_transform_in_place_100", |b| {
        let mat = Mat2::rotation(0.785);
        let mut vertices: Vec<Vec2> = (0..100)
            .map(|i| Vec2::new(i as f32, i as f32))
            .collect();

        b.iter(|| {
            mat.transform_in_place(black_box(&mut vertices));
        });
    });
}

criterion_group!(benches,
    bench_vec2_add,
    bench_vec2_mul,
    bench_mat2_transform,
    bench_mat2_batch_transform,
    bench_mat2_transform_in_place
);
criterion_main!(benches);
