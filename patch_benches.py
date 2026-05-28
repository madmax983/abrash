import re
import os

benches = ["benches/object_culling.rs", "benches/scene_render.rs"]

for bench in benches:
    with open(bench, "r") as f:
        content = f.read()

    # Prepend DrawList instantiation before the bench loops
    # This requires looking at the surrounding context manually for each.
