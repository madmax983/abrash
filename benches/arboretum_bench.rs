use abrash::experimental::arboretum::LSystem;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn arboretum_benchmark(c: &mut Criterion) {
    c.bench_function("arboretum_expand_15", |b| {
        let mut lsys = LSystem::new("A", 90.0, 1.0, 0.1);
        lsys.add_rule('A', "AB");
        lsys.add_rule('B', "A");
        b.iter(|| black_box(lsys.expand(15).unwrap()));
    });
}

criterion_group!(benches, arboretum_benchmark);
criterion_main!(benches);
