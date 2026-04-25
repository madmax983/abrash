import os

filepath = 'Cargo.toml'
with open(filepath, 'r') as f:
    content = f.read()

# Add demo to [[example]] section
demo_block = """
[[example]]
name = "cel_shader_demo"
path = "examples/cel_shader_demo.rs"
required-features = ["nova"]
"""

# Add bench to [[bench]] section
bench_block = """
[[bench]]
name = "cel_shader_bench"
harness = false
required-features = ["nova"]
"""

if 'name = "cel_shader_demo"' not in content:
    content += demo_block
if 'name = "cel_shader_bench"' not in content:
    content += bench_block

with open(filepath, 'w') as f:
    f.write(content)
