use abrash::experimental::posterize::{PosterizeConfig, apply_posterize};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_posterize(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient to process
    for y in 0..height {
        for x in 0..width {
            let r = (x % 256) << 16;
            let g = (y % 256) << 8;
            let b = (x + y) % 256;
            fb.set_pixel(x as i32, y as i32, 0xFF00_0000 | r | g | b);
        }
    }

    let config = PosterizeConfig { levels: 4.0 };

    c.bench_function("apply_posterize_800x600", |b| {
        b.iter(|| {
            apply_posterize(black_box(&mut fb), black_box(&config));
        });
    });
}

fn bench_posterize_extreme(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient to process
    for y in 0..height {
        for x in 0..width {
            let r = (x % 256) << 16;
            let g = (y % 256) << 8;
            let b = (x + y) % 256;
            fb.set_pixel(x as i32, y as i32, 0xFF00_0000 | r | g | b);
        }
    }

    let config = PosterizeConfig { levels: 16.0 };

    c.bench_function("apply_posterize_1920x1080", |b| {
        b.iter(|| {
            apply_posterize(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_posterize, bench_posterize_extreme);
criterion_main!(benches);
