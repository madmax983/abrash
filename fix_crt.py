with open("src/experimental/crt.rs", "r") as f:
    lines = f.readlines()

new_lines = []
for line in lines:
    if line.startswith("use std::cell::RefCell;"):
        pass # remove from top
    else:
        new_lines.append(line)

# find where "use crate::framebuffer::Framebuffer;" is and put it before that
for i, line in enumerate(new_lines):
    if line.startswith("use crate::framebuffer::Framebuffer;"):
        new_lines.insert(i, "use std::cell::RefCell;\n")
        break

with open("src/experimental/crt.rs", "w") as f:
    f.writelines(new_lines)
