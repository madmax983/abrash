import re

with open("crates/abrash-render/src/render_api/cpu_renderer.rs", "r") as f:
    content = f.read()

search_block = """        draw_list.clear_color = frame.clear_color;
        draw_list.lights.clone_from(&frame.lights);"""

replace_block = """        draw_list.clear_color = frame.clear_color;

        // ⚡ Bolt: Use `clear()` instead of `clone_from` or `std::mem::take` to retain
        // the capacity of the lights vector, eliminating heap allocations for identical frames.
        draw_list.lights.clear();
        draw_list.lights.extend_from_slice(&frame.lights);"""

content = content.replace(search_block, replace_block)

with open("crates/abrash-render/src/render_api/cpu_renderer.rs", "w") as f:
    f.write(content)

print("Patched cpu_renderer.rs")
