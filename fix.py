with open("src/rasterizer/tile.rs", "r") as f:
    lines = f.readlines()

new_lines = []
skip = False

# Remove the incomplete duplicate closure specifically.
for i, line in enumerate(lines):
    if i == 2341 and "tiles.par_iter().for_each_init(" in line:
        pass
    if i == 2340 and "(0..self.tiles_y)" in line:
        skip = True

    if skip and "tiles.par_iter().for_each_init(" in line:
        skip = False

    if not skip:
        new_lines.append(line)

with open("src/rasterizer/tile.rs.tmp", "w") as f:
    f.writelines(new_lines)
