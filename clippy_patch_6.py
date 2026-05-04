import re

with open("crates/abrash-render/src/experimental/physarum.rs", "r") as f:
    code = f.read()

# Make the changes in Step 2 of physarum.rs
new_step_2 = """                // Step 2: Diffuse and decay
                let diffuse_center = 1.0 - config.diffuse_rate;
                let diffuse_side = config.diffuse_rate / 8.0;

                for y in 0..height {
                    let y_prev = if y == 0 { height - 1 } else { y - 1 };
                    let y_next = if y == height - 1 { 0 } else { y + 1 };

                    let row_idx = y * width;
                    let row_prev_idx = y_prev * width;
                    let row_next_idx = y_next * width;

                    for x in 0..width {
                        let x_prev = if x == 0 { width - 1 } else { x - 1 };
                        let x_next = if x == width - 1 { 0 } else { x + 1 };

                        let sum =
                            // Top row
                            trail_borrow[row_prev_idx + x_prev] * diffuse_side +
                            trail_borrow[row_prev_idx + x] * diffuse_side +
                            trail_borrow[row_prev_idx + x_next] * diffuse_side +
                            // Middle row
                            trail_borrow[row_idx + x_prev] * diffuse_side +
                            trail_borrow[row_idx + x] * diffuse_center +
                            trail_borrow[row_idx + x_next] * diffuse_side +
                            // Bottom row
                            trail_borrow[row_next_idx + x_prev] * diffuse_side +
                            trail_borrow[row_next_idx + x] * diffuse_side +
                            trail_borrow[row_next_idx + x_next] * diffuse_side;

                        // Decay
                        let decayed = (sum - config.decay_rate).max(0.0);
                        trail_next_borrow[row_idx + x] = decayed;
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
