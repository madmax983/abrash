import sys

with open("src/experimental/directional_blur.rs", "r") as f:
    text = f.read()

text = text.replace("""    let process_row = |(y, row): (usize, &mut [u32])| {
        for (x, pixel) in row.iter_mut().enumerate().take(width) {
            let mut r_sum = 0;
            let mut g_sum = 0;
            let mut b_sum = 0;

            // Initialize fixed point coords with an offset of 32768 (0.5 in 16.16)
            // This provides free mathematical rounding when we shift right later.
            let mut fx = (x as i32) << 16;
            fx += 32768;
            let mut fy = (y as i32) << 16;
            fy += 32768;

            for _ in 0..config.num_samples {
                // Extract integer part by shifting right 16 bits.
                // Because of the 0.5 offset, this is equivalent to round()
                let px = fx >> 16;
                let py = fy >> 16;

                // Clamp to edges
                let px = px.clamp(0, width as i32 - 1) as usize;
                let py = py.clamp(0, height as i32 - 1) as usize;

                let color = source_pixels[py * width + px];
                let r = (color >> 16) & 0xFF;
                let g = (color >> 8) & 0xFF;
                let b = color & 0xFF;

                r_sum += r;
                g_sum += g;
                b_sum += b;

                // Advance sample positions
                fx += dx_step;
                fy += dy_step;
            }

            let final_r = ((r_sum as f32) * inv_samples).min(255.0) as u32;
            let final_g = ((g_sum as f32) * inv_samples).min(255.0) as u32;
            let final_b = ((b_sum as f32) * inv_samples).min(255.0) as u32;

        if source_pixels.len() != fb_slice.len() {
            source_pixels.resize(fb_slice.len(), 0);
        }
        source_pixels.copy_from_slice(fb_slice);
        // We now safely reference the slice. We can extract it as an immutable reference
        // to pass into the parallel iterator safely since `RefMut` doesn't implement `Sync`.
        let source_slice: &[u32] = &source_pixels;""", """    let dx_step_fixed = (config.dx * inv_samples * 65536.0) as i32;
    let dy_step_fixed = (config.dy * inv_samples * 65536.0) as i32;
    let inv_samples_fixed = (inv_samples * 65536.0) as u32;

    SOURCE_PIXELS.with(|source_pixels_cell| {
        let mut source_pixels = source_pixels_cell.borrow_mut();

        let fb_slice = framebuffer.as_slice();
        if source_pixels.len() != fb_slice.len() {
            source_pixels.resize(fb_slice.len(), 0);
        }
        source_pixels.copy_from_slice(fb_slice);
        // We now safely reference the slice. We can extract it as an immutable reference
        // to pass into the parallel iterator safely since `RefMut` doesn't implement `Sync`.
        let source_slice: &[u32] = &source_pixels;""")

text = text.replace("""    // Pre-calculate steps in 16.16 fixed point format
    let dx_step = (config.dx * inv_samples * 65536.0) as i32;
    let dy_step = (config.dy * inv_samples * 65536.0) as i32;

""", """    // Pre-calculate steps in 16.16 fixed point format
""")


with open("src/experimental/directional_blur.rs", "w") as f:
    f.write(text)
