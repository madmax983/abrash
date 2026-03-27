use abrash::experimental::arboretum::LSystem;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn arboretum_benchmark(c: &mut Criterion) {
    c.bench_function("arboretum_expand_15", |b| {
        let mut lsys = LSystem::new("A", 90.0, 1.0, 0.1);
        lsys.add_rule('A', "AB");
        lsys.add_rule('B', "A");
        b.iter(|| black_box(lsys.expand(15).unwrap()));
    });

    // Benchmark successful generation
    c.bench_function("arboretum_generate_mesh_normal", |b| {
        let mut lsys = LSystem::new("F", 90.0, 1.0, 0.1);
        lsys.add_rule('F', "F[+F]F[-F]F");
        b.iter(|| black_box(lsys.generate_mesh(4).unwrap()));
    });

    // Benchmark fast-failing OOM prevention
    c.bench_function("arboretum_generate_mesh_fast_fail", |b| {
        let mut lsys = LSystem::new("[", 90.0, 1.0, 0.1);
        lsys.add_rule('[', "[[[[[[[[[["); // 10x growth per iteration
        b.iter(|| {
            // We expect an error due to the 10_000 stack depth limit being exceeded.
            // 5 iterations: 10^5 = 100,000 characters, all '['
            let res = lsys.generate_mesh(5);
            assert!(res.is_err());
            black_box(res)
        });
    });
}

criterion_group!(benches, arboretum_benchmark);
criterion_main!(benches);
