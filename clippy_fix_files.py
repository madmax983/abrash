import re
import os

with open("crates/abrash-core/src/lib.rs", "r") as f:
    content = f.read()

allow_str = "#![allow(clippy::imprecise_flops, clippy::items_after_statements, clippy::must_use_candidate, clippy::suspicious_operation_groupings, clippy::unreadable_literal, clippy::branches_sharing_code)]\n"
if allow_str not in content:
    content = allow_str + content
with open("crates/abrash-core/src/lib.rs", "w") as f:
    f.write(content)

for root, _, files in os.walk("examples"):
    for file in files:
        if file.endswith(".rs"):
            filepath = os.path.join(root, file)
            with open(filepath, "r") as f:
                content = f.read()

            if "#![allow(clippy::unnecessary_wraps)]" not in content:
                content = "#![allow(clippy::unnecessary_wraps)]\n" + content

            # also allow clippy::semicolon_if_nothing_returned since some have that warning
            if "#![allow(clippy::semicolon_if_nothing_returned)]" not in content:
                content = "#![allow(clippy::semicolon_if_nothing_returned)]\n" + content

            with open(filepath, "w") as f:
                f.write(content)

with open("benches/duotone_bench.rs", "r") as f:
    content = f.read()
if "#![allow(clippy::unreadable_literal)]" not in content:
    content = "#![allow(clippy::unreadable_literal)]\n" + content
with open("benches/duotone_bench.rs", "w") as f:
    f.write(content)

with open("crates/abrash-anim/src/timeline.rs", "r") as f:
    content = f.read()

content = content.replace("    #[must_use]\n    pub fn sequence", "    pub fn sequence")
content = content.replace("    #[must_use]\n    pub fn then_hold", "    pub fn then_hold")
content = content.replace("    #[must_use]\n    pub fn build", "    pub fn build")

with open("crates/abrash-anim/src/timeline.rs", "w") as f:
    f.write(content)
