use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "nova")]
use abrash::experimental::kaleidoscope::apply_kaleidoscope;

#[cfg(feature = "nova")]
fn bench_kaleidoscope(c: &mut Criterion) {
    let mut group = c.benchmark_group("kaleidoscope");

    // Create a dummy framebuffer with some simple pattern
    let width = 640;
    let height = 480;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with something other than solid color to make caching realistic
    for y in 0..height {
        for x in 0..width {
            let color = (x ^ y) as u32;
            fb.set_pixel(x as i32, y as i32, 0xFF000000 | color);
        }
    }

    group.bench_function("apply_kaleidoscope/6_segments/640x480", |b| {
        b.iter(|| {
            apply_kaleidoscope(black_box(&mut fb), black_box(6));
        });
    });

    group.finish();
}

#[cfg(not(feature = "nova"))]
fn bench_kaleidoscope(c: &mut Criterion) {
    let mut group = c.benchmark_group("kaleidoscope");
    group.bench_function("stub", |b| b.iter(|| black_box(0)));
    group.finish();
}

criterion_group!(benches, bench_kaleidoscope);
criterion_main!(benches);
