with open("crates/abrash-render/src/experimental/posterize.rs", "r") as f:
    code = f.read()

# I see the `*pixel = a | (new_r << 16) | (new_g << 8) | new_b;` line but the setup is the old one?
# Wait, did the python script run incorrectly? Let's fix the entire function.

new_function = """pub fn apply_posterize(fb: &mut Framebuffer, config: &PosterizeConfig) {
    let levels = config.levels.max(2.0); // Minimum of 2 levels
    let levels_minus_1 = levels - 1.0;
    let levels_minus_1_int = levels_minus_1.max(1.0) as u32;

    // Use chunks_exact_mut to eliminate bounds checking and option unwrapping
    let width = fb.width() as usize;
    for row in fb.as_mut_slice().chunks_exact_mut(width) {
        for pixel in row.iter_mut() {
            let p = *pixel;
            // Extract channels
            let a = p & 0xFF00_0000;
            let r = (p >> 16) & 0xFF;
            let g = (p >> 8) & 0xFF;
            let b = p & 0xFF;

            // ⚡ Bolt: Replace floating-point arithmetic and round() casting with fast, pure integer arithmetic.
            let new_r = (((r * levels_minus_1_int + 127) / 255) * 255) / levels_minus_1_int;
            let new_g = (((g * levels_minus_1_int + 127) / 255) * 255) / levels_minus_1_int;
            let new_b = (((b * levels_minus_1_int + 127) / 255) * 255) / levels_minus_1_int;

            *pixel = a | (new_r << 16) | (new_g << 8) | new_b;
        }
    }
}"""

import re
code = re.sub(r'pub fn apply_posterize\(fb: &mut Framebuffer, config: &PosterizeConfig\) \{.+?        }\n    }\n\}', new_function, code, flags=re.DOTALL)

with open("crates/abrash-render/src/experimental/posterize.rs", "w") as f:
    f.write(code)

