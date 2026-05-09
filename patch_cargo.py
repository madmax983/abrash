import re

with open('Cargo.toml', 'r') as f:
    content = f.read()

bench_entry = """
[[bench]]
name = "color_swar_bench"
harness = false
"""

if "color_swar_bench" not in content:
    content += bench_entry

with open('Cargo.toml', 'w') as f:
    f.write(content)
