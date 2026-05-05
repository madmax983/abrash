import re

with open("crates/abrash-render/src/experimental/physarum.rs", "r") as f:
    code = f.read()

# Make the changes in Step 3 of physarum.rs
new_step_3 = """                // Step 3: Render trail map to framebuffer
                let fb_slice = fb.as_mut_slice();
                let trail_color_rgb = config.trail_color & 0x00_FF_FF_FF;
                for i in 0..grid_size {
                    let intensity = trail_borrow[i];
                    if intensity > 0.01 {
                        // Map intensity to alpha 0-255
                        let alpha = (intensity * 255.0).clamp(0.0, 255.0) as u32;
                        let color_with_alpha = (alpha << 24) | trail_color_rgb;

                        let bg = Color::from_argb_u32(fb_slice[i]);
                        let fg = Color::from_argb_u32(color_with_alpha);

                        let blend = Color::blend_over(fg, bg);

                        fb_slice[i] = blend.to_argb_u32();
                    }
                }"""

code = re.sub(
    r"                // Step 3: Render trail map to framebuffer.*?fb_slice\[i\] = blend\.to_argb_u32\(\);\n                    }\n                }",
    new_step_3,
    code,
    flags=re.DOTALL
)

with open("crates/abrash-render/src/experimental/physarum.rs", "w") as f:
    f.write(code)
