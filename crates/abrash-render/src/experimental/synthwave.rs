use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;

pub fn apply_synthwave(fb: &mut Framebuffer, zb: &ZBuffer) {
    if fb.width() != zb.width() || fb.height() != zb.height() {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let horizon = height / 2;

    let depths = zb.as_slice();

    for (y, row) in fb.as_mut_slice().chunks_exact_mut(width).enumerate() {
        if y > horizon {
            // Ground Grid
            for (x, pixel) in row.iter_mut().enumerate() {
                // Perspective division simulation
                let depth = depths[y * width + x];
                if depth == f32::INFINITY {
                    *pixel = 0xFF00_0000;
                    continue;
                }

                let is_grid_line = y % 10 == 0 || x % 10 == 0;

                if is_grid_line {
                    // Magenta grid lines, darkened by distance
                    let intensity = (255.0 / (1.0 + depth * 0.1)) as u32;
                    let color = (intensity.min(255) << 16) | (intensity.min(255));
                    *pixel = 0xFF00_0000 | color;
                } else {
                    // Dark purple ground
                    *pixel = 0xFF10_0020;
                }
            }
        } else {
            // Sky Gradient (Orange to Dark Blue)
            let t = y as f32 / horizon as f32; // 0.0 at top, 1.0 at horizon

            // At horizon (t=1.0) -> Orange (255, 128, 0)
            // At top (t=0.0) -> Dark Blue (0, 0, 64)
            let r = (t * 255.0) as u32;
            let g = (t * 128.0) as u32;
            let b = ((1.0 - t) * 64.0) as u32;

            let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

            for pixel in row.iter_mut() {
                *pixel = color;
            }
        }
    }
}
