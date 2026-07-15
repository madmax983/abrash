with open('crates/abrash-render/src/procedural.rs', 'r') as f:
    content = f.read()

bad_str = """/// Generates a plasma effect.
///
/// Returns an error if the texture dimensions are invalid.
///
/// # Errors
/// Returns an error if `width` or `height` is zero.
///
/// # Examples
///
/// ```
/// use abrash_render::procedural::plasma;
///
/// let tex = plasma(32, 32).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
static PLASMA_LUT: std::sync::OnceLock<[u32; 1024]> = std::sync::OnceLock::new();

pub fn plasma"""

good_str = """static PLASMA_LUT: std::sync::OnceLock<[u32; 1024]> = std::sync::OnceLock::new();

/// Generates a plasma effect.
///
/// Returns an error if the texture dimensions are invalid.
///
/// # Errors
/// Returns an error if `width` or `height` is zero.
///
/// # Examples
///
/// ```
/// use abrash_render::procedural::plasma;
///
/// let tex = plasma(32, 32).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
pub fn plasma"""

content = content.replace(bad_str, good_str)

with open('crates/abrash-render/src/procedural.rs', 'w') as f:
    f.write(content)
