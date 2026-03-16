use abrash::framebuffer::Framebuffer;
use abrash::post_process::median::apply_median_filter;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rand::Rng;

fn median_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let mut rng = rand::thread_rng();

    // Add random noise
    for p in fb.as_mut_slice() {
        *p = rng.r#gen();
    }

    c.bench_function("apply_median_filter_1080p", |b| {
        b.iter(|| apply_median_filter(black_box(&mut fb)));
    });
}

criterion_group!(benches, median_benchmark);
criterion_main!(benches);
