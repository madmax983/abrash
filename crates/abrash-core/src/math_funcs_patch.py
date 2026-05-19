import re
with open("crates/abrash-core/src/math/funcs.rs", "r") as f:
    content = f.read()

content = content.replace("iter.map(|p| Vec3::new(p[0], p[1], p[2])).collect()", "/* removed */")
# Let's check for iter.collect() inside crates/abrash-core
