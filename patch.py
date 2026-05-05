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

with open("crates/abrash-render/src/experimental/physarum.rs", "w") as f:
    f.write(code)
