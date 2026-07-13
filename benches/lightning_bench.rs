use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::lightning::{LightningConfig, LightningFilter};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn lightning_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let mut lightning = LightningFilter::new(LightningConfig::default());

    c.bench_function("lightning_filter 1920x1080", |b| {
        b.iter(|| {
            lightning.apply(black_box(&mut fb));
        });
    });
}

criterion_group!(benches, lightning_benchmark);
criterion_main!(benches);
