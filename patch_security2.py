with open('tests/security_scanline_bounds.rs', 'r') as f:
    text = f.read()

text = text.replace(
"""    draw_scanline_gouraud(
        &mut fb,
        &mut zb,
        -1, // Invalid Y
        0,
        50,
        1.0,
        (0, 0, 0),
        0.0,
        (0, 0, 0),
    );""",
"""    draw_scanline_gouraud(
        &mut fb,
        &mut zb,
        -1, // Invalid Y
        0,
        50,
        GouraudSpanStartI64 { z_start: 1.0, c_start: (0, 0, 0) },
        &GouraudGradients { dz_dx: 0.0, dc_dx: (0, 0, 0) },
    );"""
)

text = text.replace(
"""    draw_scanline_gouraud(
        &mut fb,
        &mut zb,
        100, // Invalid Y
        0,
        50,
        1.0,
        (0, 0, 0),
        0.0,
        (0, 0, 0),
    );""",
"""    draw_scanline_gouraud(
        &mut fb,
        &mut zb,
        100, // Invalid Y
        0,
        50,
        GouraudSpanStartI64 { z_start: 1.0, c_start: (0, 0, 0) },
        &GouraudGradients { dz_dx: 0.0, dc_dx: (0, 0, 0) },
    );"""
)

with open('tests/security_scanline_bounds.rs', 'w') as f:
    f.write(text)
