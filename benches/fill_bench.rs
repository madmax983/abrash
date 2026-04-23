use abrash_core::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_fill_slice(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let color = 0xFFFF_FFFF;

    let start_idx = 100;
    let end_idx = 400;

    c.bench_function("fill_slice_safe", |b| {
        b.iter(|| {
            let slice = fb.as_mut_slice();
            slice[black_box(start_idx)..=black_box(end_idx)].fill(black_box(color));
        });
    });

    c.bench_function("fill_slice_unchecked", |b| {
        b.iter(|| {
            let slice = fb.as_mut_slice();
            unsafe {
                slice
                    .get_unchecked_mut(black_box(start_idx)..=black_box(end_idx))
                    .fill(black_box(color));
            }
        });
    });
}

criterion_group!(benches, bench_fill_slice);
criterion_main!(benches);
