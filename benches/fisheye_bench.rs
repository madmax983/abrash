use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[cfg(feature = "nova")]
use abrash::experimental::fisheye::apply_fisheye;
use abrash::framebuffer::Framebuffer;

#[cfg(feature = "nova")]
fn bench_fisheye(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFF_AA_BB_CC);

    c.bench_function("fisheye_800x600", |b| {
        b.iter(|| {
            apply_fisheye(black_box(&mut fb), black_box(0.5));
        });
    });
}

#[cfg(not(feature = "nova"))]
fn bench_fisheye(_c: &mut Criterion) {}

criterion_group!(benches, bench_fisheye);
criterion_main!(benches);
