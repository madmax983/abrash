use abrash_render::experimental::physarum::PhysarumSim;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_physarum_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("Physarum");

    let mut sim = PhysarumSim::new(800, 600);
    // Add 10,000 agents
    for _ in 0..10_000 {
        sim.add_agent(400.0, 300.0, 0.0);
    }

    group.bench_function("step_10k", |b| {
        b.iter(|| {
            sim.step();
        })
    });

    group.finish();
}

criterion_group!(benches, bench_physarum_step);
criterion_main!(benches);
