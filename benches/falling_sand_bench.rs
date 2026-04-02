use abrash::experimental::falling_sand::{Cell, FallingSand};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rand::Rng;

fn falling_sand_benchmark(c: &mut Criterion) {
    let width = 800;
    let height = 600;

    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFF000000); // Clear color

    let mut sand = FallingSand::new(width as usize, height as usize);
    let mut rng = rand::thread_rng();

    // Randomly fill ~10% of the grid with sand
    let sand_color = 0xFFEEDD82;
    for _ in 0..(width * height / 10) {
        let x = rng.gen_range(0..width) as usize;
        let y = rng.gen_range(0..height) as usize;
        let idx = y * width as usize + x;
        sand.grid[idx] = Cell::Sand(sand_color);
    }

    c.bench_function("falling_sand_update_and_draw", |b| {
        b.iter(|| {
            sand.update_and_draw(black_box(&mut fb), black_box(0xFF000000));
        });
    });
}

criterion_group!(benches, falling_sand_benchmark);
criterion_main!(benches);
