use abrash::math::{Vec3, Vec4};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn calculate_ts_light_normalize(n: Vec3, t: Vec4, light_dir: Vec3) -> Vec3 {
    let n_norm = n.normalize();
    let t_norm = Vec3::new(t.x, t.y, t.z).normalize();
    let t_ortho = (t_norm - n_norm * n_norm.dot(t_norm)).normalize();
    let b_ortho = n_norm.cross(t_ortho) * t.w;

    let l_world = light_dir * -1.0;

    Vec3::new(
        t_ortho.dot(l_world),
        b_ortho.dot(l_world),
        n_norm.dot(l_world),
    )
}

fn calculate_ts_light_fast_normalize(n: Vec3, t: Vec4, light_dir: Vec3) -> Vec3 {
    let n_norm = n.fast_normalize();
    let t_norm = Vec3::new(t.x, t.y, t.z).fast_normalize();
    let t_ortho = (t_norm - n_norm * n_norm.dot(t_norm)).fast_normalize();
    let b_ortho = n_norm.cross(t_ortho) * t.w;

    let l_world = light_dir * -1.0;

    Vec3::new(
        t_ortho.dot(l_world),
        b_ortho.dot(l_world),
        n_norm.dot(l_world),
    )
}

fn bench_tangent_space(c: &mut Criterion) {
    let mut group = c.benchmark_group("tangent_space_calculation");

    let n = Vec3::new(0.1, 0.9, 0.2);
    let t = Vec4::new(0.8, -0.1, 0.0, 1.0);
    let light_dir = Vec3::new(0.0, -1.0, -1.0).normalize();

    group.bench_function("normalize", |b| {
        b.iter(|| {
            black_box(calculate_ts_light_normalize(
                black_box(n),
                black_box(t),
                black_box(light_dir),
            ))
        });
    });

    group.bench_function("fast_normalize", |b| {
        b.iter(|| {
            black_box(calculate_ts_light_fast_normalize(
                black_box(n),
                black_box(t),
                black_box(light_dir),
            ))
        });
    });

    group.finish();
}

criterion_group!(benches, bench_tangent_space);
criterion_main!(benches);
