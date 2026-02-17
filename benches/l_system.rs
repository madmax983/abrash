use abrash::procedural::l_system::LSystem;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_l_system_mesh_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("l_system");

    // Define a 3D tree L-System
    // Axiom: X
    // X -> F[&+X]F[->X]+X
    // F -> FF
    // This grows fast.
    let mut lsys = LSystem::new("X", 25.0, 1.0, 0.1);
    lsys.add_rule('X', "F[&+X]F[->X]+X");
    lsys.add_rule('F', "FF");

    // Iteration 5 should produce a decent mesh.
    // Let's verify size first locally if needed, but for bench we just run it.

    group.bench_function("generate_mesh_iter_5", |b| {
        b.iter(|| {
            let mesh = lsys.generate_mesh(5);
            // Prevent optimization
            criterion::black_box(mesh);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_l_system_mesh_generation);
criterion_main!(benches);
