import re

with open("crates/abrash-render/src/experimental/physarum.rs", "r") as f:
    code = f.read()

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
