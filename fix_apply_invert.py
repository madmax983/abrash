import re

with open("crates/abrash-render/src/post_process/filters.rs", "r") as f:
    lines = f.readlines()

out = []
skip = False
for i, line in enumerate(lines):
    if i >= 239 and i <= 249:
        continue
    out.append(line)

with open("crates/abrash-render/src/post_process/filters.rs", "w") as f:
    f.writelines(out)
