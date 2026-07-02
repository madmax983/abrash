import re

with open('crates/abrash-render/src/rasterizer/tile.rs', 'r') as f:
    content = f.read()

content = content.replace('// f32_to_bits_ordered perfettamente', '// f32_to_bits_ordered perfectly')

with open('crates/abrash-render/src/rasterizer/tile.rs', 'w') as f:
    f.write(content)
