import sys

filepath = "crates/abrash-render/src/render_api/cpu_renderer.rs"
with open(filepath, "r") as f:
    content = f.read()

old_code = """
            let batches = batches?;
            draw_list.batches.extend(batches);
"""

new_code = """
            let batches = batches?;
            draw_list.batches.extend(batches);
"""

# Let's fix this up properly to not allocate a Vec if we don't need to but rayon collect into a Result<Vec> allocates.
# Wait, let's keep it simple. It's fine for correctness to allocate the `batches` Vec,
# as `cpu_mesh.mesh` and `transform_points_uninit` dominate. But we can just use
# `draw_list.batches.extend(batches);`. Actually, even better would be not to allocate, but `collect` does that.
