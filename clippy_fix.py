with open("crates/abrash-render/src/procedural.rs", "r") as f:
    code = f.read()

# Remove the incorrectly placed doc comment we added earlier
code = code.replace("/// Creates a procedural plasma texture.\n///\n/// # Errors\n/// Returns an error if the specified texture dimensions are 0.", "/// Creates a procedural plasma texture.")

# Move the actual comment to below PLASMA_LUT
search = """static PLASMA_LUT: std::sync::OnceLock<[u32; 1024]> = std::sync::OnceLock::new();

pub fn plasma(width: u32, height: u32) -> Result<Texture, &'static str> {"""

replace = """static PLASMA_LUT: std::sync::OnceLock<[u32; 1024]> = std::sync::OnceLock::new();

/// Creates a procedural plasma texture.
///
/// # Errors
///
/// Returns an error if the texture dimensions are zero or invalid.
pub fn plasma(width: u32, height: u32) -> Result<Texture, &'static str> {"""

code = code.replace(search, replace)
with open("crates/abrash-render/src/procedural.rs", "w") as f:
    f.write(code)
