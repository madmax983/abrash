import os
import re

with open('src/rasterizer/tile.rs', 'r') as f:
    content = f.read()

content = content.replace("let row_end = (clear_y_max + 1)\n                                        .min(tile_y0 + TILE_SIZE as i32)\n                                        .min(height as i32);", "let row_end = (clear_y_max as u32 + 1)\n                                        .min(tile_y0 as u32 + TILE_SIZE)\n                                        .min(height);")

# Handle any remaining instances
content = content.replace("let row_end = (clear_y_max + 1)\n                                    .min(tile_y0 + TILE_SIZE as i32)\n                                    .min(height as i32);", "let row_end = (clear_y_max as u32 + 1)\n                                    .min(tile_y0 as u32 + TILE_SIZE)\n                                    .min(height);")

with open('src/rasterizer/tile.rs', 'w') as f:
    f.write(content)

