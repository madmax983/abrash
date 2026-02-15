fn transpose_naive(src: &[u32], dest: &mut [u32], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..width {
            dest[x * height + y] = src[y * width + x];
        }
    }
}

fn transpose_tiled(src: &[u32], dest: &mut [u32], width: usize, height: usize) {
    const TILE_SIZE: usize = 32;
    for y_start in (0..height).step_by(TILE_SIZE) {
        for x_start in (0..width).step_by(TILE_SIZE) {
            let y_end = (y_start + TILE_SIZE).min(height);
            let x_end = (x_start + TILE_SIZE).min(width);
            for y in y_start..y_end {
                for x in x_start..x_end {
                    dest[x * height + y] = src[y * width + x];
                }
            }
        }
    }
}

fn main() {
    let width = 1920;
    let height = 1080;
    let src = vec![1u32; width * height];
    let mut dest = vec![0u32; width * height];

    let start = std::time::Instant::now();
    for _ in 0..10 {
        transpose_tiled(&src, &mut dest, width, height);
    }
    let dur = start.elapsed();
    println!("Transpose 1080p (10 iter) took: {:?} (Avg: {:?})", dur, dur / 10);
}
