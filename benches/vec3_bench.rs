use abrash::math::Vec3;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_vec3_cross(c: &mut Criterion) {
    c.bench_function("vec3_cross", |b| {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);
        b.iter(|| black_box(v1.cross(v2)));
    });
}

fn bench_vec3_dot(c: &mut Criterion) {
    c.bench_function("vec3_dot", |b| {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);
        b.iter(|| black_box(v1.dot(v2)));
    });
}

fn bench_vec3_fast_normalize(c: &mut Criterion) {
    let mut inputs = Vec::with_capacity(1000);
    for i in 0..1000 {
        let f = i as f32;
        inputs.push(Vec3::new(f, f + 1.0, f + 2.0));
    }

    c.bench_function("vec3_fast_normalize", |b| {
        let mut i = 0;
        b.iter(|| {
            let v = unsafe { *inputs.get_unchecked(i % 1000) };
            i += 1;
            black_box(v.fast_normalize())
        });
    });
}

fn bench_vec3_normalize(c: &mut Criterion) {
    let mut inputs = Vec::with_capacity(1000);
    for i in 0..1000 {
        let f = i as f32;
        inputs.push(Vec3::new(f, f + 1.0, f + 2.0));
    }

    c.bench_function("vec3_normalize", |b| {
        let mut i = 0;
        b.iter(|| {
            let v = unsafe { *inputs.get_unchecked(i % 1000) };
            i += 1;
            black_box(v.normalize())
        });
    });
}

fn bench_vec3_add(c: &mut Criterion) {
    c.bench_function("vec3_add", |b| {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);
        b.iter(|| black_box(v1 + v2));
    });
}

criterion_group!(
    benches,
    bench_vec3_cross,
    bench_vec3_dot,
    bench_vec3_fast_normalize,
    bench_vec3_normalize,
    bench_vec3_add
);
criterion_main!(benches);
