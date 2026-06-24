use criterion::{criterion_group, criterion_main, Criterion};
use abrash_render::experimental::lsystem::LSystem;

fn bench_lsystem(c: &mut Criterion) {
    let mut lsys = LSystem::new("A");
    lsys.add_rule('A', "AB");
    lsys.add_rule('B', "A");

    c.bench_function("lsystem_expand_15", |b| {
        b.iter(|| {
            lsys.expand(15).unwrap();
        });
    });
}

criterion_group!(benches, bench_lsystem);
criterion_main!(benches);
