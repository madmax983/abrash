use abrash_core::bam::Bam;
use abrash_raycast::cast_rays_batch;
use abrash_raycast::{cast_los, cast_ray, cast_ray_detailed};
use abrash_raycast::ArrayGridMap;
use abrash_raycast::{Cell, Vec2Fixed};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Build a maze-like map with border walls and checkerboard interior pillars.
fn make_maze(size: u32) -> ArrayGridMap {
    let mut map = ArrayGridMap::new(size, size);
    // Border walls
    for x in 0..size {
        map.set(x, 0, Cell::Solid(1));
        map.set(x, size - 1, Cell::Solid(1));
    }
    for y in 0..size {
        map.set(0, y, Cell::Solid(1));
        map.set(size - 1, y, Cell::Solid(1));
    }
    // Checkerboard interior for depth complexity
    for y in 2..size - 2 {
        for x in 2..size - 2 {
            if x % 3 == 0 && y % 3 == 0 {
                map.set(x, y, Cell::Solid(1));
            }
        }
    }
    map
}

fn single_cast_ray(c: &mut Criterion) {
    let map = make_maze(64);
    let origin = Vec2Fixed::from_f32(1.5, 1.5);
    let angle = Bam::ZERO;

    c.bench_function("cast_ray 64x64", |b| {
        b.iter(|| black_box(cast_ray(&map, black_box(origin), black_box(angle))));
    });
}

fn single_cast_ray_detailed(c: &mut Criterion) {
    let map = make_maze(64);
    let origin = Vec2Fixed::from_f32(1.5, 1.5);
    let angle = Bam::ZERO;

    c.bench_function("cast_ray_detailed 64x64", |b| {
        b.iter(|| black_box(cast_ray_detailed(&map, black_box(origin), black_box(angle))));
    });
}

fn single_cast_los_diagonal(c: &mut Criterion) {
    let map = make_maze(64);
    let from = Vec2Fixed::from_f32(1.5, 1.5);
    let to = Vec2Fixed::from_f32(62.5, 62.5);

    c.bench_function("cast_los diagonal 64x64", |b| {
        b.iter(|| black_box(cast_los(&map, black_box(from), black_box(to))));
    });
}

fn batch_1000_rays(c: &mut Criterion) {
    let map = make_maze(64);
    let origin = Vec2Fixed::from_f32(32.0, 32.0);

    // 1000 rays fanning out in a full circle.
    let rays: Vec<(Vec2Fixed, Bam)> = (0..1000)
        .map(|i| {
            let angle = Bam(((i as u64) * 4_294_967_296 / 1000) as u32);
            (origin, angle)
        })
        .collect();

    let mut results = vec![None; rays.len()];

    c.bench_function("batch 1000 rays 64x64", |b| {
        b.iter(|| {
            cast_rays_batch(&map, black_box(&rays), &mut results);
            black_box(&results);
        });
    });
}

fn map_size_scaling(c: &mut Criterion) {
    let sizes: &[u32] = &[16, 64, 128, 256];
    let mut group = c.benchmark_group("cast_ray map scaling");

    for &size in sizes {
        let map = make_maze(size);
        // Origin near the center so the ray has room to travel.
        let center = (size as f32) / 2.0;
        let origin = Vec2Fixed::from_f32(center, center);
        let angle = Bam::ZERO;

        group.bench_function(format!("{size}x{size}"), |b| {
            b.iter(|| black_box(cast_ray(&map, black_box(origin), black_box(angle))));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    single_cast_ray,
    single_cast_ray_detailed,
    single_cast_los_diagonal,
    batch_1000_rays,
    map_size_scaling,
);
criterion_main!(benches);
