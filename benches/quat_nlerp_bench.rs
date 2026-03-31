use abrash_core::math::Vec3;
use abrash_core::quat::Quat;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::f32::consts::FRAC_PI_2;

fn bench_nlerp(c: &mut Criterion) {
    let q1 = Quat::identity();
    let q2 = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);

    c.bench_function("quat_nlerp", |b| {
        b.iter(|| {
            let res = black_box(q1).nlerp(black_box(&q2), black_box(0.5));
            black_box(res);
        });
    });
}

criterion_group!(benches, bench_nlerp);
criterion_main!(benches);
