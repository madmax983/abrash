use abrash::experimental::voronoi::{DistanceMetric, VoronoiFilter};
use abrash::framebuffer::{Framebuffer, ImageExporter};
use abrash::time::Instant;

fn main() {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    let start = Instant::now();

    // Apply Voronoi Filter
    let filter = VoronoiFilter::new(100, 42)
        .with_metric(DistanceMetric::Manhattan)
        .with_borders(true);

    filter.apply(&mut fb);

    let duration = start.elapsed();
    println!("Voronoi generation took: {:?}", duration);

    // Save output
    fb.export_ppm("voronoi_demo.ppm").unwrap();
    println!("Saved to voronoi_demo.ppm");
}
