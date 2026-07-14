use abrash_render::experimental::lsystem::LSystem;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn lsystem_benchmark(c: &mut Criterion) {
    c.bench_function("lsystem_expand_15", |b| {
        let mut lsys = LSystem::new("A");
        lsys.add_rule('A', "AB");
        lsys.add_rule('B', "A");
        lsys.set_max_capacity(1_000_000);
        b.iter(|| black_box(lsys.expand(15).unwrap()));
    });
}

criterion_group!(benches, lsystem_benchmark);
criterion_main!(benches);
