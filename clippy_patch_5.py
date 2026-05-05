import re

with open("crates/abrash-render/src/experimental/physarum.rs", "r") as f:
    code = f.read()

# Make the changes in Step 1 sensor loop
new_sensor = """                    let sense = |angle_offset: f32, trail_slice: &[f32]| -> f32 {
                        let sensor_angle = agent.angle + angle_offset;
                        let (sin_a, cos_a) = sensor_angle.sin_cos();
                        let sensor_pos_x = agent.position.x + cos_a * config.sensor_offset_dist;
                        let sensor_pos_y = agent.position.y + sin_a * config.sensor_offset_dist;

                        let sx = sensor_pos_x as i32;
                        let sy = sensor_pos_y as i32;

                        let mut sum = 0.0;
                        let size = config.sensor_size;
                        for dy in -size..=size {
                            let mut ny = sy + dy;
                            if ny < 0 {
                                ny += height as i32;
                            } else if ny >= height as i32 {
                                ny -= height as i32;
                            }
                            let row_idx = ny as usize * width;

                            for dx in -size..=size {
                                let mut nx = sx + dx;
                                if nx < 0 {
                                    nx += width as i32;
                                } else if nx >= width as i32 {
                                    nx -= width as i32;
                                }

                                sum += trail_slice[row_idx + nx as usize];
                            }
                        }
                        sum
                    };"""

code = re.sub(
    r"                    let sense = \|angle_offset: f32, trail_slice: &\[f32\]\| -> f32 \{.*?sum\n                    };",
    new_sensor,
    code,
    flags=re.DOTALL
)

# Make the changes in Step 1 wrap section of physarum.rs
new_wrap = """                    // Wrap position around screen (using if/else since range is known)
                    if agent.position.x < 0.0 {
                        agent.position.x += width as f32;
                    } else if agent.position.x >= width as f32 {
                        agent.position.x -= width as f32;
                    }

                    if agent.position.y < 0.0 {
                        agent.position.y += height as f32;
                    } else if agent.position.y >= height as f32 {
                        agent.position.y -= height as f32;
                    }

                    // Deposit pheromone"""

code = re.sub(
    r"                    // Wrap position around screen.*?// Deposit pheromone",
    new_wrap,
    code,
    flags=re.DOTALL
)

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
