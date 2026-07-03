use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::datamosh::{DatamoshFilter, DatamoshConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_datamosh(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = DatamoshConfig {
        intensity: 1.0,
        threshold: 5.0,
        force_iframe: false,
    };

    c.bench_function("datamosh_800x600", |b| {
        b.iter(|| {
            let mut filter = DatamoshFilter::new(); filter.apply(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_datamosh);
criterion_main!(benches);
