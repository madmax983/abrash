use abrash::experimental::voronoi::{DistanceMetric, VoronoiFilter};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn voronoi_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let filter = VoronoiFilter::new(64, 12345)
        .with_metric(DistanceMetric::Euclidean)
        .with_borders(true);

    c.bench_function("voronoi_euclidean", |b| b.iter(|| filter.apply(&mut fb)));

    let filter_manhattan = VoronoiFilter::new(64, 12345)
        .with_metric(DistanceMetric::Manhattan)
        .with_borders(false);

    c.bench_function("voronoi_manhattan", |b| {
        b.iter(|| filter_manhattan.apply(&mut fb))
    });
}

criterion_group!(benches, voronoi_benchmark);
criterion_main!(benches);
