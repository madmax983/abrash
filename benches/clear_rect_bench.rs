use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

// Copy the original un-optimized one for comparison
pub fn clear_rect_original(fb: &mut Framebuffer, x: i32, y: i32, width: u32, height: u32, color: u32) {
    if width == 0 || height == 0 {
        return;
    }

    let x1 = x;
    let y1 = y;

    // Prevent overflow when adding width to x
    // Use i64 for intermediate calculation to avoid wrapping
    let x2_i64 = i64::from(x) + i64::from(width);
    let y2_i64 = i64::from(y) + i64::from(height);

    let x2 = if x2_i64 > i64::from(i32::MAX) {
        i32::MAX
    } else {
        x2_i64 as i32
    };
    let y2 = if y2_i64 > i64::from(i32::MAX) {
        i32::MAX
    } else {
        y2_i64 as i32
    };

    let fb_width = fb.width();
    let fb_height = fb.height();

    let start_x = x1.clamp(0, fb_width as i32) as u32;
    let start_y = y1.clamp(0, fb_height as i32) as u32;
    let end_x = x2.clamp(0, fb_width as i32) as u32;
    let end_y = y2.clamp(0, fb_height as i32) as u32;

    let pixels = fb.as_mut_slice();
    for row in start_y..end_y {
        let start = (row * fb_width + start_x) as usize;
        let end = (row * fb_width + end_x) as usize;
        pixels[start..end].fill(color);
    }
}

pub fn clear_rect_optimized(width_buf: u32, height_buf: u32, pixels: &mut [u32], x: i32, y: i32, width: u32, height: u32, color: u32) {
    if width == 0 || height == 0 {
        return;
    }

    let end_x_u64 = x as i64 + width as i64;
    let end_y_u64 = y as i64 + height as i64;

    let start_x = x.clamp(0, width_buf as i32) as u32;
    let start_y = y.clamp(0, height_buf as i32) as u32;
    let end_x = end_x_u64.clamp(0, width_buf as i64) as u32;
    let end_y = end_y_u64.clamp(0, height_buf as i64) as u32;

    if start_x >= end_x || start_y >= end_y {
        return;
    }

    let buf_width = width_buf as usize;
    if start_x == 0 && end_x == width_buf {
        let start = start_y as usize * buf_width;
        let end = end_y as usize * buf_width;
        pixels[start..end].fill(color);
    } else {
        let start = start_x as usize;
        let len = (end_x - start_x) as usize;
        let start_idx = start_y as usize * buf_width;
        let end_idx = end_y as usize * buf_width;
        for row in pixels[start_idx..end_idx].chunks_exact_mut(buf_width) {
            row[start..start + len].fill(color);
        }
    }
}

fn bench_clear_rect(c: &mut Criterion) {
    let mut fb_original = Framebuffer::new(1920, 1080).unwrap();
    let mut fb_optimized = Framebuffer::new(1920, 1080).unwrap();
    let color = 0xFFFF_FFFF;

    let width_buf = fb_original.width();
    let height_buf = fb_original.height();

    let mut group = c.benchmark_group("clear_rect");

    // Clear inner rectangle
    group.bench_function("clear_rect_partial_original", |b| {
        b.iter(|| {
            clear_rect_original(
                &mut fb_original,
                black_box(460), black_box(240), black_box(1000), black_box(600), black_box(color)
            );
        });
    });

    group.bench_function("clear_rect_partial_optimized", |b| {
        let pixels = fb_optimized.as_mut_slice();
        b.iter(|| {
            clear_rect_optimized(
                black_box(width_buf), black_box(height_buf), pixels,
                black_box(460), black_box(240), black_box(1000), black_box(600), black_box(color)
            );
        });
    });

    // Clear full frame
    group.bench_function("clear_rect_full_original", |b| {
        b.iter(|| {
            clear_rect_original(
                &mut fb_original,
                black_box(0), black_box(0), black_box(width_buf), black_box(height_buf), black_box(color)
            );
        });
    });

    group.bench_function("clear_rect_full_optimized", |b| {
        let pixels = fb_optimized.as_mut_slice();
        b.iter(|| {
            clear_rect_optimized(
                black_box(width_buf), black_box(height_buf), pixels,
                black_box(0), black_box(0), black_box(width_buf), black_box(height_buf), black_box(color)
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_clear_rect);
criterion_main!(benches);
