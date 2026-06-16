import re

file_path = "crates/abrash-render/src/rasterizer/texture.rs"

with open(file_path, "r") as f:
    content = f.read()

# I accidentally added it twice in the python script. Let's fix that.
content = re.sub(r'#\[derive\(Clone, Copy\)\]\n/// Start of a perspective correct texture span\.\n#\[derive\(Clone, Copy\)\]', '/// Start of a perspective correct texture span.\n#[derive(Clone, Copy)]', content)

with open(file_path, "w") as f:
    f.write(content)
