use abrash::experimental::lsystem::{LSystem, Turtle};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn lsystem_mesh_benchmark(c: &mut Criterion) {
    let mut lsys = LSystem::new("A");
    lsys.add_rule('A', "F[+A][-A]");
    lsys.set_max_capacity(1_000_000);
    let expanded = lsys.expand(6).unwrap();

    c.bench_function("lsystem_mesh_generate", |b| {
        b.iter(|| {
            let mut turtle = Turtle::new();
            black_box(turtle.generate_mesh(&expanded).unwrap())
        });
    });
}

criterion_group!(benches, lsystem_mesh_benchmark);
criterion_main!(benches);
