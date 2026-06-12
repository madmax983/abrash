use criterion::{criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::post_process::median::{apply_median_filter, MedianFilterConfig};
use rand::Rng;

fn bench_median_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("Median Filter");

    let resolutions = [(320, 240), (800, 600)];

    let mut rng = rand::thread_rng();

    for (w, h) in resolutions {
        let mut fb = Framebuffer::new(w, h).unwrap();
        // Add random noise
        for y in 0..h {
            for x in 0..w {
                fb.set_pixel(x as i32, y as i32, 0xFF00_0000 | rng.r#gen::<u32>() & 0x00FF_FFFF);
            }
        }

        let config = MedianFilterConfig { radius: 1 };

        group.bench_function(format!("{w}x{h}"), |b| {
            b.iter(|| {
                apply_median_filter(&mut fb, &config);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_median_filter);
criterion_main!(benches);
