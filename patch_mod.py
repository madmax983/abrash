import os

filepath = 'crates/abrash-render/src/experimental/mod.rs'
with open(filepath, 'r') as f:
    content = f.read()

# Add pub mod cel_shader; alphabetically if possible, or just before pub mod cloth;
if 'pub mod cel_shader;' not in content:
    content = content.replace('pub mod cloth;', 'pub mod cel_shader;\npub mod cloth;')
    with open(filepath, 'w') as f:
        f.write(content)
