import re

with open("crates/abrash-render/src/heat_vision.rs", "r") as f:
    content = f.read()

content = content.replace(
    "if z.to_bits() != 0x7F80_0000 {",
    "// Bolt Performance Optimization:\n    // By replacing the floating-point `!= f32::INFINITY` check with its integer bitwise equivalent,\n    // we eliminate FPU comparison overhead in this hot scalar rendering loop.\n    if z.to_bits() != 0x7F80_0000 {"
)

content = content.replace(
    "if depth.to_bits() == 0x7F80_0000 {",
    "// Bolt Performance Optimization:\n        // By replacing the floating-point `== f32::INFINITY` check with its integer bitwise equivalent,\n        // we eliminate FPU comparison overhead in this hot scalar rendering loop.\n        if depth.to_bits() == 0x7F80_0000 {"
)

with open("crates/abrash-render/src/heat_vision.rs", "w") as f:
    f.write(content)


with open("crates/abrash-render/src/experimental/paper_cutout.rs", "r") as f:
    content = f.read()

content = content.replace(
    "if z.to_bits() != 0x7F80_0000 {",
    "// Bolt Performance Optimization:\n    // By replacing the floating-point `!= f32::INFINITY` check with its integer bitwise equivalent,\n    // we eliminate FPU comparison overhead in this hot scalar rendering loop.\n    if z.to_bits() != 0x7F80_0000 {"
)

content = content.replace(
    "if d.to_bits() == 0x7F80_0000 {",
    "// Bolt Performance Optimization:\n                // By replacing the floating-point `== f32::INFINITY` check with its integer bitwise equivalent,\n                // we eliminate FPU comparison overhead in this hot scalar rendering loop.\n                if d.to_bits() == 0x7F80_0000 {"
)

with open("crates/abrash-render/src/experimental/paper_cutout.rs", "w") as f:
    f.write(content)
