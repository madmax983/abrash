use abrash_render::experimental::lsystem::LSystem;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_lsystem_expand_ascii(c: &mut Criterion) {
    let mut lsys = LSystem::new("A");
    lsys.add_rule('A', "AB");
    lsys.add_rule('B', "A");

    c.bench_function("lsystem_expand_ascii", |b| {
        b.iter(|| {
            // Expand 15 times to generate a reasonably large string
            black_box(lsys.expand(black_box(15)).unwrap());
        })
    });
}

fn bench_lsystem_expand_utf8(c: &mut Criterion) {
    // We include a UTF-8 character in the axiom so it takes the safe `.chars()` fallback path
    let mut lsys = LSystem::new("螃");
    lsys.add_rule('螃', "F螃");

    c.bench_function("lsystem_expand_utf8", |b| {
        b.iter(|| {
            // Expand 15 times to generate a reasonably large string
            black_box(lsys.expand(black_box(15)).unwrap());
        })
    });
}

criterion_group!(
    benches,
    bench_lsystem_expand_ascii,
    bench_lsystem_expand_utf8
);
criterion_main!(benches);
