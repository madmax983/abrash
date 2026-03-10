with open('src/rasterizer/tile.rs', 'r') as f:
    lines = f.readlines()

out = []
skip = False
for i, line in enumerate(lines):
    if line.strip() == 'tiles.par_iter().for_each_init(':
        skip = True
    elif skip and line.strip() == 'let tile_cols = (tile_x_end - tile_x0) as usize;':
        skip = False
        continue

    if not skip:
        out.append(line)

with open('src/rasterizer/tile.rs', 'w') as f:
    f.writelines(out)
