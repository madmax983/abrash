use abrash::clipping::{clip_triangle_to_frustum, clip_triangle_to_frustum_iter};
use abrash::math::Vec3;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_clipping_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("Clipping Return Overhead");

    // Case 1: Trivial Accept
    group.bench_function("return_struct_trivial", |b| {
        let v0 = (Vec3::new(0.0, 0.0, 0.0), 1.0);
        let v1 = (Vec3::new(0.5, 0.0, 0.0), 1.0);
        let v2 = (Vec3::new(0.0, 0.5, 0.0), 1.0);
        #[allow(deprecated)]
        b.iter(|| {
            let clipped = clip_triangle_to_frustum(black_box(v0), black_box(v1), black_box(v2), |v| *v);
            let mut count = 0;
            for i in 0..clipped.count {
                let _ = black_box(clipped[i]);
                count += 1;
            }
            count
        });
    });

    group.bench_function("callback_trivial", |b| {
        let v0 = (Vec3::new(0.0, 0.0, 0.0), 1.0);
        let v1 = (Vec3::new(0.5, 0.0, 0.0), 1.0);
        let v2 = (Vec3::new(0.0, 0.5, 0.0), 1.0);
        b.iter(|| {
            let mut count = 0;
            clip_triangle_to_frustum_iter(black_box(v0), black_box(v1), black_box(v2), |v| *v, |t0, t1, t2| {
                black_box(t0);
                black_box(t1);
                black_box(t2);
                count += 1;
            });
            count
        });
    });

    // Case 2: Actual Clipping
    group.bench_function("return_struct_clipped", |b| {
        let v0 = (Vec3::new(0.0, 0.0, 1.0), 1.0);
        let v1 = (Vec3::new(0.0, 2.0, -1.0), -1.0);
        let v2 = (Vec3::new(2.0, 0.0, -1.0), -1.0);
        #[allow(deprecated)]
        b.iter(|| {
            let clipped = clip_triangle_to_frustum(black_box(v0), black_box(v1), black_box(v2), |v| *v);
            let mut count = 0;
            for i in 0..clipped.count {
                // Mimic access pattern
                let idx = i * 3;
                let _ = black_box(clipped[idx]);
                let _ = black_box(clipped[idx+1]);
                let _ = black_box(clipped[idx+2]);
                count += 1;
            }
            count
        });
    });

    group.bench_function("callback_clipped", |b| {
        let v0 = (Vec3::new(0.0, 0.0, 1.0), 1.0);
        let v1 = (Vec3::new(0.0, 2.0, -1.0), -1.0);
        let v2 = (Vec3::new(2.0, 0.0, -1.0), -1.0);
        b.iter(|| {
            let mut count = 0;
            clip_triangle_to_frustum_iter(black_box(v0), black_box(v1), black_box(v2), |v| *v, |t0, t1, t2| {
                black_box(t0);
                black_box(t1);
                black_box(t2);
                count += 1;
            });
            count
        });
    });

    group.finish();
}

criterion_group!(benches, bench_clipping_overhead);
criterion_main!(benches);
