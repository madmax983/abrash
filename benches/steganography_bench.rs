use criterion::{criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::steganography::{encode_message, decode_message};

fn bench_steganography(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill buffer
    fb.clear(0xFF_FF_FF_FF);

    let message = "A".repeat(1000); // 1000 byte message

    c.bench_function("stego_encode_1000", |b| b.iter(|| {
        encode_message(&mut fb, &message).unwrap();
    }));

    encode_message(&mut fb, &message).unwrap();

    c.bench_function("stego_decode_1000", |b| b.iter(|| {
        let _ = decode_message(&fb).unwrap();
    }));
}

criterion_group!(benches, bench_steganography);
criterion_main!(benches);
