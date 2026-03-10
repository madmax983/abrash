with open('src/rasterizer/tile.rs', 'r') as f:
    c = f.read()
import re
for match in re.finditer(r'while curr != u32::MAX', c):
    idx = match.start()
    last_open = c.rfind('/*', 0, idx)
    last_close = c.rfind('*/', 0, idx)
    is_inside = last_open > last_close
    print(f"Index: {idx}, Inside block comment: {is_inside}")
