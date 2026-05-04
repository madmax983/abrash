import re

with open("crates/abrash-render/src/experimental/physarum.rs", "r") as f:
    code = f.read()

# Make the changes in Step 2 of physarum.rs
new_step_2 = """                // Step 2: Diffuse and decay
                let diffuse_center = 1.0 - config.diffuse_rate;
                let diffuse_side = config.diffuse_rate / 8.0;

                for y in 0..height {
                    for x in 0..width {
                        let mut sum = 0.0;

                        for dy in -1..=1 {
                            let mut ny = y as i32 + dy;
                            if ny < 0 {
                                ny += height as i32;
                            } else if ny >= height as i32 {
                                ny -= height as i32;
                            }
                            let row_idx = ny as usize * width;

                            for dx in -1..=1 {
                                let mut nx = x as i32 + dx;
                                if nx < 0 {
                                    nx += width as i32;
                                } else if nx >= width as i32 {
                                    nx -= width as i32;
                                }

                                let weight = if dx == 0 && dy == 0 {
                                    diffuse_center
                                } else {
                                    diffuse_side
                                };

                                sum += trail_borrow[row_idx + nx as usize] * weight;
                            }
                        }

                        // Decay
                        let decayed = (sum - config.decay_rate).max(0.0);
                        trail_next_borrow[y * width + x] = decayed;
                    }
                }"""

code = re.sub(
    r"                // Step 2: Diffuse and decay.*?trail_next_borrow\[y \* width \+ x\] = decayed;\n                    }\n                }",
    new_step_2,
    code,
    flags=re.DOTALL
)

with open("crates/abrash-render/src/experimental/physarum.rs", "w") as f:
    f.write(code)
