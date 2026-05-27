import sys

file_path = "crates/abrash-render/src/procedural.rs"
with open(file_path, "r") as f:
    content = f.read()

new_plasma_fast = """pub fn plasma_fast(width: u32, height: u32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(width, height)?;
    if width == 0 || height == 0 {
        return Ok(tex);
    }

    // ⚡ Bolt: Pre-calculate the palette to a Look-Up Table (LUT)
    const LUT_SIZE: usize = 1024;
    let mut lut = [0u32; LUT_SIZE];
    for i in 0..LUT_SIZE {
        let normalized = i as f32 / (LUT_SIZE - 1) as f32;
        let (r_sin, _) = fast_sin_cos(normalized * std::f32::consts::PI);
        let r = (r_sin.abs() * 255.0) as u32;
        let (g_sin, _) = fast_sin_cos((normalized * std::f32::consts::PI) + 2.0);
        let g = (g_sin.abs() * 255.0) as u32;
        let (b_sin, _) = fast_sin_cos((normalized * std::f32::consts::PI) + 4.0);
        let b = (b_sin.abs() * 255.0) as u32;
        lut[i] = 0xFF00_0000 | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
    }

    let pixels = tex.pixels.as_mut_slice();

    // ⚡ Bolt: Use .chunks_exact_mut() to elide bounds checking
    for (y, row) in pixels.chunks_exact_mut(width as usize).enumerate().take(height as usize) {
        let v = y as f32;
        // ⚡ Bolt: Extract y-dependent calculations outside inner loop
        let (v2, _) = fast_sin_cos(v * 0.1);
        let v_squared = v * v;

        for (x, pixel) in row.iter_mut().enumerate() {
            let u = x as f32;
            let (v1, _) = fast_sin_cos(u * 0.1);
            let (v3, _) = fast_sin_cos((u + v) * 0.1);
            // Integer fast distance squared fallback to float due to exact math requirement
            let (v4, _) = fast_sin_cos(u.mul_add(u, v_squared).sqrt() * 0.1);

            let val = (v1 + v2 + v3 + v4) * 0.25; // -1 to 1
            let normalized = (val + 1.0) * 0.5; // 0 to 1

            // ⚡ Bolt: Replace 3 trigonometric calls with a single LUT lookup
            let lut_index = (normalized * (LUT_SIZE - 1) as f32) as usize;
            let lut_index = lut_index.clamp(0, LUT_SIZE - 1);
            *pixel = lut[lut_index];
        }
    }

    Ok(tex)
}"""

old_func = """pub fn plasma_fast(width: u32, height: u32) -> Result<Texture, &'static str> {
    plasma(width, height)
}"""

if old_func in content:
    content = content.replace(old_func, new_plasma_fast)
    with open(file_path, "w") as f:
        f.write(content)
else:
    print("Could not find old func")
