import sys

filepath = "src/platform/mod.rs"
with open(filepath, "r") as f:
    content = f.read()

content = content.replace(
    "run_windowed,",
    "run_windowed, print_error_and_exit,"
)

with open(filepath, "w") as f:
    f.write(content)
