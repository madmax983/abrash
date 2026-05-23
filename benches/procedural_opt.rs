use abrash_render::procedural::white_noise;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use abrash_core::texture::Texture;
use abrash_core::utils::XorShift32;

pub fn white_noise_original(width: u32, height: u32, seed: u32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(width, height)?;
    let mut rng = XorShift32::new(seed);
    for y in 0..height {
        for x in 0..width {
            let v = (rng.next_u32() & 0xFF) as u8;
            let color = 0xFF00_0000 | (u32::from(v) << 16) | (u32::from(v) << 8) | u32::from(v);
            tex.set_pixel(x, y, color);
        }
    }
    Ok(tex)
}

fn bench_procedural_opt(c: &mut Criterion) {
    let mut group = c.benchmark_group("Procedural Opt");
    group.bench_function("white_noise_original", |b| {
        b.iter(|| black_box(white_noise_original(256, 256, 12345).unwrap()));
    });
    group.bench_function("white_noise_optimized", |b| {
        b.iter(|| black_box(white_noise(256, 256, 12345).unwrap()));
    });
    group.finish();
}

criterion_group!(benches, bench_procedural_opt);
criterion_main!(benches);
