import re

with open("crates/abrash-render/src/post_process/filters.rs", "r") as f:
    code = f.read()

code = code.replace("use `abrash_core", "use abrash_core")
code = code.replace("use `abrash_render", "use abrash_render")

# While we're at it, let's fix any bad literals in doc tests here
code = code.replace("0xFF808080", "0xFF80_8080")
code = code.replace("0xFFFF0000", "0xFFFF_0000")
code = code.replace("0xFF000000", "0xFF00_0000")
code = code.replace("0xFFFFFFFF", "0xFFFF_FFFF")


with open("crates/abrash-render/src/post_process/filters.rs", "w") as f:
    f.write(code)
