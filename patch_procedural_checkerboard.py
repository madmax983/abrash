with open("crates/abrash-render/src/procedural.rs", "r") as f:
    content = f.read()

new_effect = """/// Generates a checkerboard texture.
///
/// # Errors
/// Returns an error if the texture dimensions are invalid or `cell_size` is 0.
///
/// # Examples
///
/// ```
/// use abrash_render::procedural::checkerboard;
///
/// let tex = checkerboard(32, 32, 8, 0xFFFFFFFF, 0xFF000000).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
pub fn checkerboard(
    width: u32,
    height: u32,
    cell_size: u32,
    color1: u32,
    color2: u32,
) -> Result<Texture, &'static str> {
    if cell_size == 0 {
        return Err("Cell size must be positive");
    }
    let mut tex = Texture::new(width, height)?;
    for y in 0..height {
        for x in 0..width {
            let cx = x / cell_size;
            let cy = y / cell_size;
            let is_color1 = (cx + cy) % 2 == 0;
            tex.set_pixel(x, y, if is_color1 { color1 } else { color2 });
        }
    }
    Ok(tex)
}

static PLASMA_LUT"""

content = content.replace("static PLASMA_LUT", new_effect)

test_patch = """    #[test]
    fn test_checkerboard() {
        let tex = checkerboard(10, 10, 5, 0xFFFFFFFF, 0xFF000000).unwrap();
        assert_eq!(tex.width(), 10);
        assert_eq!(tex.height(), 10);
        assert_eq!(tex.get_pixel(0, 0), Some(0xFFFFFFFF));
        assert_eq!(tex.get_pixel(5, 0), Some(0xFF000000));
        assert_eq!(tex.get_pixel(0, 5), Some(0xFF000000));
        assert_eq!(tex.get_pixel(5, 5), Some(0xFFFFFFFF));
    }

    #[test]
    fn test_plasma_zero_dimensions() {"""

content = content.replace("    #[test]\n    fn test_plasma_zero_dimensions() {", test_patch)

with open("crates/abrash-render/src/procedural.rs", "w") as f:
    f.write(content)
