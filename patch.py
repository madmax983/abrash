import re

with open("src/experimental/radial_blur.rs", "r") as f:
    content = f.read()

# Can we hoist `dx` calculation outside as well?
# Actually, dx increases by 1 for each pixel, so step_x increases by `step_factor * 65536.0`.
# We can do `let mut step_x = ((0.0 - cx) * step_factor * 65536.0) as i32;`
# and then add `step_factor_x` to `step_x` inside the loop!

# Wait, dx = x as f32 - cx as f32
# step_x = dx * step_factor * 65536.0 = (x - cx) * step_factor * 65536.0
# For the next pixel:
# step_x_next = (x + 1 - cx) * step_factor * 65536.0 = step_x + (step_factor * 65536.0)

# step_x increases linearly! This is even better, it replaces an f32 cast and a multiply with one integer addition!

# Let's write the patch

scalar_loop_old = """                for x in 0..width {
                    let dx = x as f32 - cx as f32;
                    let step_x = (dx * step_factor * 65536.0) as i32;

                    // Add 32768 (0.5 in 16.16 fixed-point) to naturally mathematically round on right shift
                    let mut cur_x = ((x as i32) << 16) + 32768;"""

scalar_loop_new = """                let mut step_x = (- (cx as f32) * step_factor * 65536.0) as i32;
                let step_x_inc = (step_factor * 65536.0) as i32;

                for x in 0..width {
                    // Add 32768 (0.5 in 16.16 fixed-point) to naturally mathematically round on right shift
                    let mut cur_x = ((x as i32) << 16) + 32768;"""

content = content.replace(scalar_loop_old, scalar_loop_new)

scalar_loop_old2 = """                    dest_pixels[row_start + x] = (r << 16) | (g << 8) | b;
                }"""

scalar_loop_new2 = """                    dest_pixels[row_start + x] = (r << 16) | (g << 8) | b;
                    step_x += step_x_inc;
                }"""

content = content.replace(scalar_loop_old2, scalar_loop_new2)


parallel_loop_old = """                    for (x, pixel) in row.iter_mut().enumerate() {
                        let dx = x as f32 - cx as f32;
                        let step_x = (dx * step_factor * 65536.0) as i32;

                        // Add 32768 (0.5 in 16.16 fixed-point) to naturally mathematically round on right shift
                        let mut cur_x = ((x as i32) << 16) + 32768;"""

parallel_loop_new = """                    let mut step_x = (- (cx as f32) * step_factor * 65536.0) as i32;
                    let step_x_inc = (step_factor * 65536.0) as i32;

                    for (x, pixel) in row.iter_mut().enumerate() {
                        // Add 32768 (0.5 in 16.16 fixed-point) to naturally mathematically round on right shift
                        let mut cur_x = ((x as i32) << 16) + 32768;"""

content = content.replace(parallel_loop_old, parallel_loop_new)


parallel_loop_old2 = """                        *pixel = (r << 16) | (g << 8) | b;
                    }"""

parallel_loop_new2 = """                        *pixel = (r << 16) | (g << 8) | b;
                        step_x += step_x_inc;
                    }"""

content = content.replace(parallel_loop_old2, parallel_loop_new2)

with open("src/experimental/radial_blur.rs", "w") as f:
    f.write(content)
