use abrash_core::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn clear_rect_original_unsafe(zb: &mut ZBuffer, x: i32, y: i32, width: u32, height: u32) {
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

    let start_x = x1.max(0).min(zb.width() as i32) as u32;
    let start_y = y1.max(0).min(zb.height() as i32) as u32;

    let end_x = x2.max(0).min(zb.width() as i32) as u32;
    let end_y = y2.max(0).min(zb.height() as i32) as u32;

    if start_x >= end_x || start_y >= end_y {
        return;
    }

    let w = zb.width() as usize;
    let sx = start_x as usize;
    let ex = end_x as usize;

    let sy = start_y as usize;
    let ey = end_y as usize;

    let start_idx = sy * w;
    let end_idx = ey * w;

    if sx == 0 && ex == w {
        zb.as_mut_slice()[start_idx..end_idx].fill(f32::INFINITY);
    } else {
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
}

fn bench_zbuffer_clear_rect_comparison(c: &mut Criterion) {
    let mut zb_unsafe = ZBuffer::new(1920, 1080).unwrap();
    let mut zb_safe = ZBuffer::new(1920, 1080).unwrap();

    let mut group = c.benchmark_group("zbuffer_clear_rect");

    group.bench_function("unsafe_1080p", |b| {
        b.iter(|| {
            clear_rect_original_unsafe(
                &mut zb_unsafe,
                black_box(100),
                black_box(100),
                black_box(1000),
                black_box(500),
            );
        });
    });

    group.bench_function("safe_chunking_1080p", |b| {
        b.iter(|| {
            zb_safe.clear_rect(
                black_box(100),
                black_box(100),
                black_box(1000),
                black_box(500),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_zbuffer_clear_rect_comparison);
criterion_main!(benches);
