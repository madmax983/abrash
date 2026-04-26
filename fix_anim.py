import re

with open("crates/abrash-anim/src/sequence.rs", "r") as f:
    code = f.read()

code = code.replace(
    "unsafe impl<T: Animatable> Send for Sequence<T> {}",
    "unsafe impl<T: Animatable + Send> Send for Sequence<T> {}"
)
code = code.replace(
    "unsafe impl<T: Animatable> Sync for Sequence<T> {}",
    "unsafe impl<T: Animatable + Sync> Sync for Sequence<T> {}"
)

with open("crates/abrash-anim/src/sequence.rs", "w") as f:
    f.write(code)
