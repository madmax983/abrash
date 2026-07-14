import os
import re

# 1. Move the file
os.rename("crates/abrash-core/src/animatable.rs", "crates/abrash-anim/src/animatable.rs")

# 2. Remove mod from abrash-core
with open("crates/abrash-core/src/lib.rs", "r") as f:
    content = f.read()
content = content.replace("pub mod animatable;\n", "")
with open("crates/abrash-core/src/lib.rs", "w") as f:
    f.write(content)

# 3. Add mod to abrash-anim
with open("crates/abrash-anim/src/lib.rs", "r") as f:
    content = f.read()

# Make sure it's outer doc
content = content.replace("//! Composable", "// Composable")
content = content.replace("//!\n", "//\n")
content = content.replace("//! Provides phase", "// Provides phase")
content = content.replace("//! This crate is", "// This crate is")
content = content.replace("//! `abrash_core::Animatable`.", "/// `abrash_core::Animatable`.")

# append pub mod
content = content + "\npub mod animatable;\n"

with open("crates/abrash-anim/src/lib.rs", "w") as f:
    f.write(content)

# 4. Replace usages
for root, dirs, files in os.walk("crates"):
    for file in files:
        if file.endswith(".rs"):
            path = os.path.join(root, file)
            with open(path, "r") as f:
                content = f.read()
            if "abrash_core::animatable" in content:
                content = content.replace("abrash_core::animatable", "crate::animatable")
                with open(path, "w") as f:
                    f.write(content)
