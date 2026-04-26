import re

with open("crates/abrash-anim/src/evaluable.rs", "r") as f:
    code = f.read()

code = code.replace("pub fn natural_duration(&self) -> f32", "pub const fn natural_duration(&self) -> f32")

with open("crates/abrash-anim/src/evaluable.rs", "w") as f:
    f.write(code)
