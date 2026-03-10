use abrash::experimental::lsystem::{LSystem, Turtle};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut lsys = LSystem::new("F");
    lsys.add_rule('F', "F[+F]F[-F]F");
    let commands = lsys.expand(5).unwrap();

    c.bench_function("lsystem_generate_mesh", |b| {
        b.iter(|| {
            let mut turtle = Turtle::new();
            let _mesh = turtle.generate_mesh(black_box(&commands));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
