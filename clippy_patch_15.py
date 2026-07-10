import re

with open("crates/abrash-render/src/procedural.rs", "r") as f:
    content = f.read()

content = content.replace("""/// ```
/// use abrash_render::procedural::plasma;
///
/// let tex = plasma(32, 32).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```""", """/// ```
/// use abrash_render::procedural::plasma;
///
/// let tex = plasma(32, 32).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
///
/// # Errors
///
/// Returns an error if the texture dimensions are zero, or if allocation fails.""")

with open("crates/abrash-render/src/procedural.rs", "w") as f:
    f.write(content)
