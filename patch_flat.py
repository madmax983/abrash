import re

with open("src/rasterizer/flat.rs", "r") as f:
    content = f.read()

# Let's check `draw_scanline_flat` and vectorize/unroll it.
# Actually `fill_cube_culling` runs `fill_triangle_3d` in flat.rs.
# If we check `is_backface` which was manually implemented to use `i128`,
# wait, what if we use i64 and check?
