use abrash_render::experimental::lsystem::LSystem;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_lsystem(c: &mut Criterion) {
    let mut group = c.benchmark_group("lsystem");

    group.bench_function("expand_12", |b| {
        let mut lsys = LSystem::new("A");
        lsys.add_rule('A', "AB");
        lsys.add_rule('B', "A");
        b.iter(|| {
            black_box(lsys.expand(12).unwrap());
        });
    });
}

criterion_group!(benches, bench_lsystem);
criterion_main!(benches);
