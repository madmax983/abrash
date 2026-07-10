import re

with open("crates/abrash-render/src/procedural.rs", "r") as f:
    content = f.read()

content = content.replace("/// generates a procedural plasma texture.", "/// generates a procedural plasma texture.\n///\n/// # Errors\n///\n/// Returns an error if the texture fails to allocate.")

with open("crates/abrash-render/src/procedural.rs", "w") as f:
    f.write(content)
