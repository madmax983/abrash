use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, criterion_group, criterion_main};

#[cfg(feature = "nova")]
use abrash::experimental::synthwave::{SynthwaveConfig, apply_synthwave};

#[cfg(feature = "nova")]
fn bench_synthwave(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let config = SynthwaveConfig::default();

    c.bench_function("apply_synthwave_1080p", |b| {
        b.iter(|| {
            apply_synthwave(&mut fb, &config);
        });
    });
}

#[cfg(not(feature = "nova"))]
fn bench_synthwave(_c: &mut Criterion) {}

criterion_group!(benches, bench_synthwave);
criterion_main!(benches);
