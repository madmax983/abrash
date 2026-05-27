import os

file = "crates/abrash-render/src/experimental/mod.rs"
with open(file, 'r') as f:
    content = f.read()

if "pub mod error;" not in content:
    content += "\npub mod error;\n"
    with open(file, 'w') as f:
        f.write(content)
