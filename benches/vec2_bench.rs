use abrash::math::Vec2;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_vec2_ops(c: &mut Criterion) {
    let v1 = Vec2::new(3.0, 4.0);
    c.bench_function("vec2_length", |b| b.iter(|| black_box(v1).length()));
}

criterion_group!(benches, bench_vec2_ops);
criterion_main!(benches);
