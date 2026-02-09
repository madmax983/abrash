use abrash::hiz_buffer::{AABB3D, HiZBuffer};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rand::prelude::*;

fn bench_hiz_build(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;

    // Create a zbuffer with some random depth data
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let slice = zb.as_mut_slice();
    for depth in slice.iter_mut() {
        *depth = rng.gen_range(0.0..1.0);
    }

    let mut hiz = HiZBuffer::new(width, height);

    c.bench_function("hiz_build_1080p", |b| {
        b.iter(|| {
            hiz.build_pyramid(black_box(&zb));
        });
    });
}

fn bench_hiz_query(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;

    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let slice = zb.as_mut_slice();
    for depth in slice.iter_mut() {
        *depth = rng.gen_range(0.0..1.0);
    }

    let mut hiz = HiZBuffer::new(width, height);
    hiz.build_pyramid(&zb);

    // Generate some random AABBs
    let mut aabbs = Vec::new();
    for _ in 0..1000 {
        let min_x = rng.gen_range(0..width as i32 - 100);
        let min_y = rng.gen_range(0..height as i32 - 100);
        let w = rng.gen_range(10..100);
        let h = rng.gen_range(10..100);
        aabbs.push(AABB3D {
            min_x,
            max_x: min_x + w,
            min_y,
            max_y: min_y + h,
            min_depth: rng.gen_range(0.0..0.5),
            max_depth: rng.gen_range(0.5..1.0),
        });
    }

    c.bench_function("hiz_query_1000_aabbs", |b| {
        b.iter(|| {
            for aabb in &aabbs {
                black_box(hiz.is_potentially_visible(*aabb));
            }
        });
    });
}

criterion_group!(benches, bench_hiz_build, bench_hiz_query);
criterion_main!(benches);
