use abrash_core::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn clear_rect_current(fb: &mut Framebuffer, x: i32, y: i32, width: u32, height: u32, color: u32) {
    if width == 0 || height == 0 {
        return;
    }

    let x1 = x;
    let y1 = y;

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

    let start_x = x1.max(0).min(fb.width() as i32) as u32;
    let start_y = y1.max(0).min(fb.height() as i32) as u32;
    let end_x = x2.max(0).min(fb.width() as i32) as u32;
    let end_y = y2.max(0).min(fb.height() as i32) as u32;

    if start_x >= end_x || start_y >= end_y {
        return;
    }

    let sx = start_x as usize;
    let ex = end_x as usize;
    let sy = start_y as usize;
    let ey = end_y as usize;
    let w = fb.width() as usize;

    let start_idx = sy * w;
    let end_idx = ey * w;

    if sx == 0 && ex == w {
        fb.as_mut_slice()[start_idx..end_idx].fill(color);
    } else {
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
}

fn clear_rect_new_opt(fb: &mut Framebuffer, x: i32, y: i32, width: u32, height: u32, color: u32) {
    if width == 0 || height == 0 {
        return;
    }

    let x1 = x;
    let y1 = y;

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

    let start_x = x1.max(0).min(fb.width() as i32) as u32;
    let start_y = y1.max(0).min(fb.height() as i32) as u32;
    let end_x = x2.max(0).min(fb.width() as i32) as u32;
    let end_y = y2.max(0).min(fb.height() as i32) as u32;

    if start_x >= end_x || start_y >= end_y {
        return;
    }

    let sx = start_x as usize;
    let ex = end_x as usize;
    let sy = start_y as usize;
    let ey = end_y as usize;
    let w = fb.width() as usize;

    let start_idx = sy * w;
    let end_idx = ey * w;

    if sx == 0 && ex == w {
        fb.as_mut_slice()[start_idx..end_idx].fill(color);
    } else {
        for row in fb.as_mut_slice()[start_idx..end_idx].chunks_exact_mut(w) {
            unsafe {
                row.get_unchecked_mut(sx..ex).fill(color);
            }
        }
    }
}

fn bench_clear_rect_opt(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let mut group = c.benchmark_group("clear_rect_opt");

    group.bench_function("current_1080p", |b| {
        b.iter(|| {
            clear_rect_current(
                &mut fb,
                black_box(100),
                black_box(100),
                black_box(1000),
                black_box(500),
                black_box(0xFFFF_FFFF),
            );
        });
    });

    group.bench_function("new_opt_1080p", |b| {
        b.iter(|| {
            clear_rect_new_opt(
                &mut fb,
                black_box(100),
                black_box(100),
                black_box(1000),
                black_box(500),
                black_box(0xFFFF_FFFF),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_clear_rect_opt);
criterion_main!(benches);
