with open('benches/gouraud_scanline.rs', 'r') as f:
    text = f.read()

text = text.replace(
    "use abrash_render::rasterizer::{draw_scanline_gouraud};",
    "use abrash_render::rasterizer::{draw_scanline_gouraud};\nuse abrash_render::rasterizer::gouraud::{GouraudSpanStartI64, GouraudGradients};"
)

text = text.replace(
"""                black_box(z_start),
                black_box(c_start),
                black_box(dz_dx),
                black_box(dc_dx),""",
"""                black_box(GouraudSpanStartI64 { z_start, c_start }),
                black_box(&GouraudGradients { dz_dx, dc_dx }),"""
)

with open('benches/gouraud_scanline.rs', 'w') as f:
    f.write(text)
