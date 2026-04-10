use abrash::experimental::reaction_diffusion::ReactionDiffusion;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_reaction_diffusion(c: &mut Criterion) {
    let mut sim = ReactionDiffusion::new(100, 100);
    // Seed some initial spots to ensure the simulation isn't purely empty/fast-pathed.
    sim.seed(50, 50, 10);

    let mut group = c.benchmark_group("reaction_diffusion");
    group.bench_function("step_100x100", |b| {
        b.iter(|| {
            sim.step();
        });
    });
    group.finish();
}

criterion_group!(benches, bench_reaction_diffusion);
criterion_main!(benches);
