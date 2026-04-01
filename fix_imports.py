import os

modules = [
    "clipping", "culling", "framebuffer", "geometry", "hiz_buffer",
    "math", "mesh", "obj_loader", "texture", "time", "utils", "zbuffer"
]

def process_file(filepath):
    with open(filepath, "r") as f:
        content = f.read()

    new_content = content
    for mod in modules:
        # We look for "crate::mod" and replace it with "abrash_core::mod"
        # However, it could be part of a block import `use crate::{math, ...}`
        # Let's start with basic replacements first.
        new_content = new_content.replace(f"crate::{mod}", f"abrash_core::{mod}")

    if new_content != content:
        with open(filepath, "w") as f:
            f.write(new_content)
        print(f"Updated {filepath}")

for root, _, files in os.walk("crates/abrash-render/src"):
    for file in files:
        if file.endswith(".rs"):
            process_file(os.path.join(root, file))
