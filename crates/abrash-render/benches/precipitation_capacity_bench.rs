use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[derive(Clone, Copy)]
struct Drop {
    x: f32,
    y: f32,
    z: f32,
    vx: f32,
    vy: f32,
    life: f32,
    color: u32,
}

fn bench_no_reserve(c: &mut Criterion) {
    c.bench_function("precipitation_no_reserve", |b| {
        b.iter(|| {
            let mut drops = Vec::new();
            while drops.len() < 100_000 {
                drops.push(Drop {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                    vx: 1.0,
                    vy: 1.0,
                    life: 1.0,
                    color: 0,
                });
            }
            black_box(drops);
        });
    });
}

fn bench_with_reserve(c: &mut Criterion) {
    c.bench_function("precipitation_with_reserve", |b| {
        b.iter(|| {
            let mut drops = Vec::new();
            drops.reserve_exact(100_000usize.saturating_sub(drops.len()));
            while drops.len() < 100_000 {
                drops.push(Drop {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                    vx: 1.0,
                    vy: 1.0,
                    life: 1.0,
                    color: 0,
                });
            }
            black_box(drops);
        });
    });
}

criterion_group!(benches, bench_no_reserve, bench_with_reserve);
criterion_main!(benches);
