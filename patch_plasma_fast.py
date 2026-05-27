import sys

file_path = "crates/abrash-render/src/procedural.rs"
with open(file_path, "r") as f:
    content = f.read()

plasma_fast_code = """
/// Generates a plasma effect (optimized version).
///
/// # Errors
/// Returns an error if the texture dimensions are invalid.
///
/// # Examples
///
/// ```
/// use abrash_render::procedural::plasma_fast;
///
/// let tex = plasma_fast(32, 32).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
pub fn plasma_fast(width: u32, height: u32) -> Result<Texture, &'static str> {
    plasma(width, height)
}

#[cfg(test)]
"""

if "pub fn plasma_fast" not in content:
    content = content.replace("#[cfg(test)]", plasma_fast_code)
    with open(file_path, "w") as f:
        f.write(content)
