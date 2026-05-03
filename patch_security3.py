with open('tests/security_scanline_bounds.rs', 'r') as f:
    text = f.read()

text = text.replace(
    "use abrash_render::rasterizer::{draw_scanline_flat, draw_scanline_gouraud, draw_scanline_textured_perspective, PerspectiveSpanStart, PerspectiveTextureGradients};",
    "use abrash_render::rasterizer::{draw_scanline_flat, draw_scanline_gouraud, draw_scanline_textured_perspective, PerspectiveSpanStart, PerspectiveTextureGradients};\nuse abrash_render::rasterizer::gouraud::{GouraudSpanStartI64, GouraudGradients};"
)

# Also fix the duplicate import I injected earlier
text = text.replace(
    "use abrash_render::rasterizer::gouraud::{GouraudSpanStartI64, GouraudGradients};\nuse abrash_render::rasterizer::gouraud::{GouraudSpanStartI64, GouraudGradients};",
    "use abrash_render::rasterizer::gouraud::{GouraudSpanStartI64, GouraudGradients};"
)


with open('tests/security_scanline_bounds.rs', 'w') as f:
    f.write(text)
