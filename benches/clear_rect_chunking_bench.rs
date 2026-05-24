use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn clear_rect_original_fb(
    fb: &mut Framebuffer,
    sx: usize,
    ex: usize,
    sy: usize,
    ey: usize,
    color: u32,
) {
    let w = fb.width() as usize;
    let start_idx = sy * w;
    let len = ex - sx;
    let mut offset = start_idx + sx;
    let slice = fb.as_mut_slice();
    for _ in sy..ey {
        unsafe {
            slice.get_unchecked_mut(offset..offset + len).fill(color);
        }
        offset += w;
    }
}

fn clear_rect_optimized_fb(
    fb: &mut Framebuffer,
    sx: usize,
    ex: usize,
    sy: usize,
    ey: usize,
    color: u32,
) {
    let w = fb.width() as usize;
    fb.as_mut_slice()[sy * w..ey * w]
        .chunks_exact_mut(w)
        .for_each(|row| row[sx..ex].fill(color));
}

fn clear_rect_original_zb(zb: &mut ZBuffer, sx: usize, ex: usize, sy: usize, ey: usize) {
    let w = zb.width() as usize;
    let start_idx = sy * w;
    let len = ex - sx;
    let mut offset = start_idx + sx;
    let slice = zb.as_mut_slice();
    for _ in sy..ey {
        unsafe {
            slice
                .get_unchecked_mut(offset..offset + len)
                .fill(f32::INFINITY);
        }
        offset += w;
    }
}

fn clear_rect_optimized_zb(zb: &mut ZBuffer, sx: usize, ex: usize, sy: usize, ey: usize) {
    let w = zb.width() as usize;
    zb.as_mut_slice()[sy * w..ey * w]
        .chunks_exact_mut(w)
        .for_each(|row| row[sx..ex].fill(f32::INFINITY));
}

fn bench_clear_rect_chunking(c: &mut Criterion) {
    let mut group = c.benchmark_group("clear_rect_chunking");

    let w = 1920;
    let h = 1080;
    let sx = 200;
    let ex = 1720;
    let sy = 100;
    let ey = 980;

    let mut fb = Framebuffer::new(w as u32, h as u32).unwrap();
    let mut zb = ZBuffer::new(w as u32, h as u32).unwrap();

    group.bench_function("fb_original", |b| {
        b.iter(|| {
            clear_rect_original_fb(
                black_box(&mut fb),
                black_box(sx),
                black_box(ex),
                black_box(sy),
                black_box(ey),
                black_box(0xFFFF_0000),
            );
        });
    });

    group.bench_function("fb_optimized", |b| {
        b.iter(|| {
            clear_rect_optimized_fb(
                black_box(&mut fb),
                black_box(sx),
                black_box(ex),
                black_box(sy),
                black_box(ey),
                black_box(0xFFFF_0000),
            );
        });
    });

    group.bench_function("zb_original", |b| {
        b.iter(|| {
            clear_rect_original_zb(
                black_box(&mut zb),
                black_box(sx),
                black_box(ex),
                black_box(sy),
                black_box(ey),
            );
        });
    });

    group.bench_function("zb_optimized", |b| {
        b.iter(|| {
            clear_rect_optimized_zb(
                black_box(&mut zb),
                black_box(sx),
                black_box(ex),
                black_box(sy),
                black_box(ey),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_clear_rect_chunking);
criterion_main!(benches);
