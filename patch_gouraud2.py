import re

with open('crates/abrash-render/src/rasterizer/gouraud.rs', 'r') as f:
    content = f.read()

# I want to check if the `pack_color_fixed_i32` logic is what we need.
# Wait, I already added `pack_color_fixed_i32` to Gouraud, let me verify that it compiles.

pass
