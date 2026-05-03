with open('benches/gouraud_scanline.rs', 'r') as f:
    text = f.read()

text = text.replace(
    "use abrash::rasterizer::draw_scanline_gouraud;",
    "use abrash::rasterizer::draw_scanline_gouraud;\nuse abrash_render::rasterizer::gouraud::{GouraudSpanStartI64, GouraudGradients};"
)

with open('benches/gouraud_scanline.rs', 'w') as f:
    f.write(text)
