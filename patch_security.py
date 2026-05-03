with open('tests/security_scanline_bounds.rs', 'r') as f:
    text = f.read()

text = text.replace(
    "use abrash_render::rasterizer::{draw_scanline_flat, draw_scanline_gouraud, draw_scanline_textured_perspective};",
    "use abrash_render::rasterizer::{draw_scanline_flat, draw_scanline_gouraud, draw_scanline_textured_perspective, PerspectiveSpanStart, PerspectiveTextureGradients};\nuse abrash_render::rasterizer::gouraud::{GouraudSpanStartI64, GouraudGradients};"
)

text = text.replace(
"""    draw_scanline_gouraud(
        &mut fb,
        &mut zb,
        0,
        0,
        99,
        1.0,
        (0, 0, 0),
        0.0,
        (0, 0, 0),
    );""",
"""    draw_scanline_gouraud(
        &mut fb,
        &mut zb,
        0,
        0,
        99,
        GouraudSpanStartI64 { z_start: 1.0, c_start: (0, 0, 0) },
        &GouraudGradients { dz_dx: 0.0, dc_dx: (0, 0, 0) },
    );"""
)

text = text.replace(
"""    draw_scanline_gouraud(
        &mut fb,
        &mut zb,
        0,
        -10,
        200,
        1.0,
        (0, 0, 0),
        0.0,
        (0, 0, 0),
    );""",
"""    draw_scanline_gouraud(
        &mut fb,
        &mut zb,
        0,
        -10,
        200,
        GouraudSpanStartI64 { z_start: 1.0, c_start: (0, 0, 0) },
        &GouraudGradients { dz_dx: 0.0, dc_dx: (0, 0, 0) },
    );"""
)

with open('tests/security_scanline_bounds.rs', 'w') as f:
    f.write(text)
