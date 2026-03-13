import re

with open("src/experimental/radial_blur.rs", "r") as f:
    content = f.read()

# Apply the two optimizations that worked!
# 1) Hoist step_y and dy out of the inner loop
# 2) Offset cur_x and cur_y by 32768, and use unsafe get_unchecked instead of branchy min()

scalar_loop_old = """            for y in 0..height {
                let row_start = y * width;
                for x in 0..width {
                    let dx = x as f32 - cx as f32;
                    let dy = y as f32 - cy as f32;

                    let step_x = (dx * step_factor * 65536.0) as i32;
                    let step_y = (dy * step_factor * 65536.0) as i32;

                    let mut cur_x = (x as i32) << 16;
                    let mut cur_y = (y as i32) << 16;

                    let mut r_acc = 0;
                    let mut g_acc = 0;
                    let mut b_acc = 0;

                    for _ in 0..samples {
                        let x_idx = (cur_x >> 16).max(0).min(w_m1) as usize;
                        let y_idx = (cur_y >> 16).max(0).min(h_m1) as usize;

                        let color = src_fb[y_idx * width + x_idx];"""

scalar_loop_new = """            for y in 0..height {
                let row_start = y * width;
                let dy = y as f32 - cy as f32;
                let step_y = (dy * step_factor * 65536.0) as i32;

                for x in 0..width {
                    let dx = x as f32 - cx as f32;
                    let step_x = (dx * step_factor * 65536.0) as i32;

                    // Add 32768 (0.5 in 16.16 fixed-point) to naturally mathematically round on right shift
                    let mut cur_x = ((x as i32) << 16) + 32768;
                    let mut cur_y = ((y as i32) << 16) + 32768;

                    let mut r_acc = 0;
                    let mut g_acc = 0;
                    let mut b_acc = 0;

                    for _ in 0..samples {
                        // Unsigned cast efficiently clamps negatives by wrapping them to a huge number,
                        // which is then caught by the single min() call.
                        let x_idx = ((cur_x >> 16) as usize).min(w_m1 as usize);
                        let y_idx = ((cur_y >> 16) as usize).min(h_m1 as usize);

                        // Unchecked access since we manually clamped within valid array bounds
                        let color = unsafe { *src_fb.get_unchecked(y_idx * width + x_idx) };"""

content = content.replace(scalar_loop_old, scalar_loop_new)

parallel_loop_old = """                .for_each(|(y, row)| {
                    for (x, pixel) in row.iter_mut().enumerate() {
                        let dx = x as f32 - cx as f32;
                        let dy = y as f32 - cy as f32;

                        let step_x = (dx * step_factor * 65536.0) as i32;
                        let step_y = (dy * step_factor * 65536.0) as i32;

                        let mut cur_x = (x as i32) << 16;
                        let mut cur_y = (y as i32) << 16;

                        let mut r_acc = 0;
                        let mut g_acc = 0;
                        let mut b_acc = 0;

                        for _ in 0..samples {
                            // Using direct min/max instead of clamp is usually faster when inline
                            let x_idx = (cur_x >> 16).max(0).min(w_m1) as usize;
                            let y_idx = (cur_y >> 16).max(0).min(h_m1) as usize;

                            let color = src_fb[y_idx * width + x_idx];"""

parallel_loop_new = """                .for_each(|(y, row)| {
                    let dy = y as f32 - cy as f32;
                    let step_y = (dy * step_factor * 65536.0) as i32;

                    for (x, pixel) in row.iter_mut().enumerate() {
                        let dx = x as f32 - cx as f32;
                        let step_x = (dx * step_factor * 65536.0) as i32;

                        // Add 32768 (0.5 in 16.16 fixed-point) to naturally mathematically round on right shift
                        let mut cur_x = ((x as i32) << 16) + 32768;
                        let mut cur_y = ((y as i32) << 16) + 32768;

                        let mut r_acc = 0;
                        let mut g_acc = 0;
                        let mut b_acc = 0;

                        for _ in 0..samples {
                            // Unsigned cast efficiently clamps negatives by wrapping them to a huge number,
                            // which is then caught by the single min() call.
                            let x_idx = ((cur_x >> 16) as usize).min(w_m1 as usize);
                            let y_idx = ((cur_y >> 16) as usize).min(h_m1 as usize);

                            // Unchecked access since we manually clamped within valid array bounds
                            let color = unsafe { *src_fb.get_unchecked(y_idx * width + x_idx) };"""

content = content.replace(parallel_loop_old, parallel_loop_new)


with open("src/experimental/radial_blur.rs", "w") as f:
    f.write(content)
