import sys

file_path = "Cargo.toml"
with open(file_path, "r") as f:
    content = f.read()

bench_entry = """
[[bench]]
name = "plasma_fast_bench"
harness = false
"""

if "plasma_fast_bench" not in content:
    with open(file_path, "a") as f:
        f.write("\n" + bench_entry)
