import re

with open("crates/abrash-render/src/experimental/jelly.rs", "r") as f:
    content = f.read()

content = content.replace("use std::collections::HashSet;", "use foldhash::{HashSet, HashSetExt};")

with open("crates/abrash-render/src/experimental/jelly.rs", "w") as f:
    f.write(content)
