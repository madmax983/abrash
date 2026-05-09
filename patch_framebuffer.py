import re

with open('crates/abrash-core/src/blitter.rs', 'r') as f:
    content = f.read()

# Replace the inner loop of `fill_rect_alpha` to use the new SWAR blend instead of `alpha_blend_pixel`
content = content.replace(
    "let dst = fb_pixels[idx];\n            let blended = alpha_blend_pixel(color, dst, alpha);\n            fb_pixels[idx] = blended;",
    "let dst = fb_pixels[idx];\n            let blended = crate::color::Color::blend_over_u32_swar(color, dst);\n            fb_pixels[idx] = blended;"
)

with open('crates/abrash-core/src/blitter.rs', 'w') as f:
    f.write(content)
