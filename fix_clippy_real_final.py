with open('crates/abrash-render/src/procedural.rs', 'r') as f:
    content = f.read()

content = content.replace(
'''static PLASMA_LUT: std::sync::OnceLock<[u32; 1024]> = std::sync::OnceLock::new();

pub fn plasma(width: u32, height: u32) -> Result<Texture, &'static str> {''',
'''static PLASMA_LUT: std::sync::OnceLock<[u32; 1024]> = std::sync::OnceLock::new();

/// Generate a plasma effect.
///
/// # Errors
/// Returns an error if the texture fails to allocate.
pub fn plasma(width: u32, height: u32) -> Result<Texture, &'static str> {'''
)

with open('crates/abrash-render/src/procedural.rs', 'w') as f:
    f.write(content)
