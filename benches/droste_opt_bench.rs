use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::droste::DrosteConfig;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::cell::RefCell;

thread_local! {
    static SOURCE_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub fn apply_droste_original(fb: &mut Framebuffer, config: &DrosteConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.iterations == 0 {
        return;
    }

    let len = width * height;

    SOURCE_BUFFER.with(|buf| {
        let mut b = buf.borrow_mut();
        if b.len() < len {
            b.resize(len, 0);
        }
        b[..len].copy_from_slice(fb.as_slice());
    });

    let dest_pixels = fb.as_mut_slice();

    let cx = width as f32 * 0.5 + config.offset_x * width as f32;
    let cy = height as f32 * 0.5 + config.offset_y * height as f32;

    SOURCE_BUFFER.with(|buf| {
        let src_pixels = buf.borrow();
        let src_pixels_slice: &[u32] = &src_pixels;

        let process_row = |y: usize, row: &mut [u32]| {
            let dy = y as f32 - cy;
            for (x, pixel) in row.iter_mut().enumerate() {
                let dx = x as f32 - cx;

                let mut sample_x = dx;
                let mut sample_y = dy;
                let mut sampled_color = src_pixels_slice[y * width + x];

                for _ in 0..config.iterations {
                    let unscaled_x = sample_x / config.scale;
                    let unscaled_y = sample_y / config.scale;

                    let px = (unscaled_x + cx).round() as i32;
                    let py = (unscaled_y + cy).round() as i32;

                    if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                        sample_x = unscaled_x;
                        sample_y = unscaled_y;
                        sampled_color = src_pixels_slice[(py as usize) * width + (px as usize)];
                    } else {
                        break;
                    }
                }

                *pixel = sampled_color;
            }
        };

        dest_pixels
            .chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| process_row(y, row));
    });
}

fn bench_droste_opt(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    for y in 0..600 {
        for x in 0..800 {
            let color = 0xFF00_0000 | (x as u32 & 0xFF) << 16 | (y as u32 & 0xFF) << 8;
            fb.set_pixel(x, y, color);
        }
    }

    let config = DrosteConfig {
        iterations: 4,
        scale: 0.5,
        offset_x: 0.0,
        offset_y: 0.0,
    };

    let mut group = c.benchmark_group("droste_opt");

    group.bench_function("original", |b| {
        b.iter(|| {
            apply_droste_original(black_box(&mut fb), black_box(&config));
        });
    });

    group.bench_function("optimized", |b| {
        b.iter(|| {
            abrash_render::experimental::droste::apply_droste(
                black_box(&mut fb),
                black_box(&config),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_droste_opt);
criterion_main!(benches);
