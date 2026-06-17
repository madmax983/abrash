use criterion::{criterion_group, criterion_main, Criterion};
use abrash_render::experimental::seam_carving::{apply_seam_carving, SeamCarvingConfig};

fn bench_seam_carving(c: &mut Criterion) {
    let mut group = c.benchmark_group("seam_carving");

    // Test on a realistic image size
    let width = 800;
    let height = 600;

    // Create an interesting pattern so all energy isn't just 0
    let mut original_buffer = vec![0u32; width * height];
    for y in 0..height {
        for x in 0..width {
            let r = (x % 256) as u32;
            let g = (y % 256) as u32;
            let b = ((x + y) % 256) as u32;
            original_buffer[y * width + x] = 0xFF000000 | (r << 16) | (g << 8) | b;
        }
    }

    group.bench_function("seam_carving_50_seams_800x600", |b| {
        let config = SeamCarvingConfig {
            seams_to_remove: 50,
            original_width: width as u32,
            stride: width as u32,
            height: height as u32,
        };

        // We must re-clone the buffer because the algorithm works in-place
        // and modifies the data, decreasing effective width.
        b.iter_batched(
            || original_buffer.clone(),
            |mut buffer| {
                apply_seam_carving(&mut buffer, config);
                buffer
            },
            criterion::BatchSize::LargeInput,
        )
    });

    group.finish();
}

criterion_group!(benches, bench_seam_carving);
criterion_main!(benches);
