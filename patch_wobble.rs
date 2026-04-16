<<<<<<< SEARCH
        // Calculate the horizontal shift for this entire row.
        // Fast float-to-int cast (as i32) is preferred over .round() in hot loops.
        let shift = (config.amplitude * crate::math::fast_sin(y_f32 * freq_scale + config.time)) as i32;

        let src_row_start = y * width;
        let src_row = &src[src_row_start..src_row_start + width];

        for (x, pixel) in row.iter_mut().enumerate() {
            // Calculate the source X coordinate.
            // If the pixel is shifted to the right, we need to read from the left.
            let src_x = x as i32 - shift;

            // Handle horizontal bounds via clamping (or wrapping, but clamping is safer).
            let clamped_src_x = src_x.clamp(0, width as i32 - 1) as usize;

            *pixel = src_row[clamped_src_x];
        }
=======
        // Calculate the horizontal shift for this entire row.
        // Fast float-to-int cast (as i32) is preferred over .round() in hot loops.
        // We use standard .sin() instead of fast_sin() for precision to ensure test stability,
        // and because this sine is computed once per row, the overhead is negligible.
        let shift = (config.amplitude * (y_f32 * freq_scale + config.time).sin()) as i32;

        let src_row_start = y * width;
        let src_row = &src[src_row_start..src_row_start + width];

        for (x, pixel) in row.iter_mut().enumerate() {
            // Calculate the source X coordinate.
            // If the pixel is shifted to the right, we need to read from the left.
            let src_x = x as i32 - shift;

            // Handle horizontal bounds via clamping (or wrapping, but clamping is safer).
            let clamped_src_x = src_x.clamp(0, width as i32 - 1) as usize;

            *pixel = src_row[clamped_src_x];
        }
>>>>>>> REPLACE
