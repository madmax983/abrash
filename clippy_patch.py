import re

with open("crates/abrash-render/src/rasterizer/tile.rs", "r") as f:
    content = f.read()

content = content.replace("struct AlignedBuffer<T> {\n    _data: Vec<T>,", "struct AlignedBuffer<T> {\n    data: Vec<T>,")
content = content.replace("self._data.", "self.data.")
content = content.replace("_data: data", "data")

with open("crates/abrash-render/src/rasterizer/tile.rs", "w") as f:
    f.write(content)
