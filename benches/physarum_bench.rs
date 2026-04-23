use abrash::experimental::physarum::{
    PhysarumAgent, PhysarumConfig, diffuse_and_evaporate, update_agents,
};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn physarum_benchmark(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let num_agents = 100_000;

    let mut agents = vec![PhysarumAgent::new(400.0, 300.0, 0.0); num_agents];
    let mut trail_map = vec![0.0; width * height];
    let mut next_trail_map = vec![0.0; width * height];
    let config = PhysarumConfig::default();

    let mut group = c.benchmark_group("Physarum");

    group.bench_function("update_agents 100k", |b| {
        b.iter(|| {
            update_agents(
                black_box(&mut agents),
                black_box(&mut trail_map),
                black_box(width),
                black_box(height),
                black_box(&config),
                black_box(42),
            );
        });
    });

    group.bench_function("diffuse_and_evaporate 800x600", |b| {
        b.iter(|| {
            diffuse_and_evaporate(
                black_box(&trail_map),
                black_box(&mut next_trail_map),
                black_box(width),
                black_box(height),
                black_box(&config),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, physarum_benchmark);
criterion_main!(benches);
