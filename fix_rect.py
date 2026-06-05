import re

with open("crates/abrash-render/src/rasterizer/rect.rs", "r") as f:
    code = f.read()

code = code.replace("""            draw_horizontal_line_unchecked(fb, cx_left, cx_right, y + height as i32 - 1, color); // Bottom""", """            draw_horizontal_line_unchecked(fb, cx_left, cx_right, y.saturating_add(h_i32.saturating_sub(1)), color); // Bottom""")

code = code.replace("""            draw_vertical_line_unchecked(fb, x + width as i32 - 1, cy_top, cy_bottom, color); // Right""", """            draw_vertical_line_unchecked(fb, x.saturating_add(w_i32.saturating_sub(1)), cy_top, cy_bottom, color); // Right""")

code = code.replace("""            draw_horizontal_line(fb, cx_left, cx_right, y + height as i32 - 1, color); // Bottom""", """            draw_horizontal_line(fb, cx_left, cx_right, y.saturating_add(h_i32.saturating_sub(1)), color); // Bottom""")

code = code.replace("""            draw_vertical_line(fb, x + width as i32 - 1, cy_top, cy_bottom, color); // Right""", """            draw_vertical_line(fb, x.saturating_add(w_i32.saturating_sub(1)), cy_top, cy_bottom, color); // Right""")


with open("crates/abrash-render/src/rasterizer/rect.rs", "w") as f:
    f.write(code)
