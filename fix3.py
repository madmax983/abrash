with open("src/rasterizer/tile.rs", "r") as f:
    lines = f.readlines()

new_lines = []
for line in lines:
    new_lines.append(line)
    if "for col in 0..tile_cols {" in line:
        pass

# I'll just use a much simpler fix. Let's start clean again.
