import re
with open("crates/abrash-core/src/transform.rs", "r") as f:
    content = f.read()

content = content.replace("let mut actual = vectors.clone();", "let mut actual = vectors;")
content = content.replace("let mut actual = points.clone();", "let mut actual = points;")

with open("crates/abrash-core/src/transform.rs", "w") as f:
    f.write(content)
