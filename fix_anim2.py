import re

# Fix hold.rs
with open("crates/abrash-anim/src/hold.rs", "r") as f:
    code = f.read()
code = code.replace("pub fn natural_duration(&self) -> f32", "pub const fn natural_duration(&self) -> f32")
with open("crates/abrash-anim/src/hold.rs", "w") as f:
    f.write(code)

# Fix keyframe.rs
with open("crates/abrash-anim/src/keyframe.rs", "r") as f:
    code = f.read()
code = code.replace("pub fn natural_duration(&self) -> f32", "pub const fn natural_duration(&self) -> f32")
with open("crates/abrash-anim/src/keyframe.rs", "w") as f:
    f.write(code)

# Fix sequence.rs missing stuff
with open("crates/abrash-anim/src/sequence.rs", "r") as f:
    code = f.read()

code = code.replace("pub fn natural_duration(&self) -> f32", "#[must_use]\n    pub const fn natural_duration(&self) -> f32")
code = code.replace("pub fn evaluate(&self, phase: f32) -> Sample<T>", "#[must_use]\n    pub fn evaluate(&self, phase: f32) -> Sample<T>")
code = code.replace("segments.iter().map(|s| s.natural_duration()).sum()", "segments.iter().map(super::evaluable::Evaluable::natural_duration).sum()")

with open("crates/abrash-anim/src/sequence.rs", "w") as f:
    f.write(code)

# Fix post_process filters.rs
with open("crates/abrash-render/src/post_process/filters.rs", "r") as f:
    code = f.read()

code = code.replace(
"""/// let mut fb = Framebuffer::new(1, 1).unwrap();
/// fb.clear(0xFFC0_C0C0); // Light Gray (192)
/// apply_solarize(&mut fb, 127);
/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F);""",
"""/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_render::post_process::apply_solarize;
/// let mut fb = Framebuffer::new(1, 1).unwrap();
/// fb.clear(0xFFC0_C0C0); // Light Gray (192)
/// apply_solarize(&mut fb, 127);
/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F);
/// ```""")
with open("crates/abrash-render/src/post_process/filters.rs", "w") as f:
    f.write(code)
