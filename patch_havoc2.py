import re

file_path = "crates/abrash-render/tests/havoc_tile_ub_test.rs"
with open(file_path, "r") as f:
    content = f.read()

# We need to make sure the entire module is cfg'ed under parallel
content = "#![cfg(feature = \"parallel\")]\n\n" + content.replace("#![cfg(feature = \"parallel\")]\n", "")

with open(file_path, "w") as f:
    f.write(content)
