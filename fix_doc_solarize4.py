import re

with open("crates/abrash-render/src/post_process/filters.rs", "r") as f:
    code = f.read()

code = code.replace("use `abrash", "use abrash")
code = code.replace("apply_solarize`;", "apply_solarize;")
code = code.replace("0xFFC0_C0C0", "0xFFC0_C0C0") # unchanged
code = code.replace("0xFF3F_3F3F", "0xFF3F_3F3F") # unchanged

code = code.replace("0xFF808080", "0xFF80_8080")
code = code.replace("0xFFFF0000", "0xFFFF_0000")
code = code.replace("0xFF000000", "0xFF00_0000")
code = code.replace("0xFFFFFFFF", "0xFFFF_FFFF")

# We should make sure we don't accidentally do things wrong. Wait, the problem earlier was `apply_invert` was redefined. Let's see what's really there.
