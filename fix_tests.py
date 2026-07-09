with open("crates/abrash-render/src/rasterizer/tile.rs", "r") as f:
    content = f.read()

content = content.replace(
    "self.tile_bins.generations[idx] == self.tile_bins.current_generation",
    "tr.tile_bins.generations[idx] == tr.tile_bins.current_generation"
)

with open("crates/abrash-render/src/rasterizer/tile.rs", "w") as f:
    f.write(content)
