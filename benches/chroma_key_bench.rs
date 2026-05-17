#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::chroma_key::smooth_chroma_key;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_chroma_key(c: &mut Criterion) {
    let mut fg = Framebuffer::new(800, 600).unwrap();
    let mut bg = Framebuffer::new(800, 600).unwrap();
    let key_color = 0xFF_00FF00; // Green screen

    // Fill half with exact green
    for y in 0..600 {
        for x in 0..400 {
            fg.set_pixel(x, y, 0xFF_00FF00);
            bg.set_pixel(x, y, 0xFF_AA_BB_CC);
        }
        for x in 400..800 {
            fg.set_pixel(x, y, 0xFF_FF0000);
            bg.set_pixel(x, y, 0xFF_AA_BB_CC);
        }
    }

    c.bench_function("chroma_key_800x600", |b| {
        b.iter(|| {
            smooth_chroma_key(
                black_box(&mut fg),
                black_box(&bg),
                black_box(key_color),
                black_box(10.0),
                black_box(50.0)
            );
        });
    });
}

criterion_group!(benches, bench_chroma_key);
criterion_main!(benches);
