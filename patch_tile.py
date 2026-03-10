import re

with open("src/rasterizer/tile.rs", "r") as f:
    content = f.read()

# Let's check where `flat_map_iter` is used.
