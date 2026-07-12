with open("crates/abrash-render/src/procedural.rs", "r") as f:
    content = f.read()

old_checkerboard = """pub fn checkerboard(
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
}"""

new_checkerboard = """pub fn checkerboard(
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
    let pixels = tex.pixels_mut();

    for y in 0..height {
        let cy = y / cell_size;
        let row_start = (y * width) as usize;
        let row_end = row_start + width as usize;

        let mut x = 0;
        let row = &mut pixels[row_start..row_end];
        for p in row.iter_mut() {
            let cx = x / cell_size;
            let is_color1 = (cx + cy) % 2 == 0;
            *p = if is_color1 { color1 } else { color2 };
            x += 1;
        }
    }
    Ok(tex)
}"""

content = content.replace(old_checkerboard, new_checkerboard)

with open("crates/abrash-render/src/procedural.rs", "w") as f:
    f.write(content)
