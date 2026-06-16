import re

file_path = "crates/abrash-render/src/rasterizer/texture.rs"

with open(file_path, "r") as f:
    content = f.read()

# Make the structs Copy and Clone
content = re.sub(r'pub struct TexSpanState', '#[derive(Clone, Copy)]\npub struct TexSpanState', content)
content = re.sub(r'pub struct TexSpanStep', '#[derive(Clone, Copy)]\npub struct TexSpanStep', content)
content = re.sub(r'pub struct GouraudSpanState', '#[derive(Clone, Copy)]\npub struct GouraudSpanState', content)
content = re.sub(r'pub struct GouraudSpanStep', '#[derive(Clone, Copy)]\npub struct GouraudSpanStep', content)
content = re.sub(r'pub struct PerspectiveSpanStart', '#[derive(Clone, Copy)]\npub struct PerspectiveSpanStart', content)

# Fix unnecessary struct building in draw_span_trilinear_simd
content = re.sub(r'TexSpanState\s*\{\s*z:\s*state\.z,\s*u_fix:\s*state\.u_fix,\s*v_fix:\s*state\.v_fix,\s*\}', 'state', content)
content = re.sub(r'TexSpanStep\s*\{\s*dz_dx:\s*step\.dz_dx,\s*du_fix:\s*step\.du_fix,\s*dv_fix:\s*step\.dv_fix,\s*\}', 'step', content)

# Fix unnecessary struct building in draw_span_textured_gouraud_simd
content = re.sub(r'TexSpanState\s*\{\s*z:\s*tex_state\.z,\s*u_fix:\s*tex_state\.u_fix,\s*v_fix:\s*tex_state\.v_fix,\s*\}', 'tex_state', content)
content = re.sub(r'TexSpanStep\s*\{\s*dz_dx:\s*tex_step\.dz_dx,\s*du_fix:\s*tex_step\.du_fix,\s*dv_fix:\s*tex_step\.dv_fix,\s*\}', 'tex_step', content)
content = re.sub(r'GouraudSpanState\s*\{\s*r_fix:\s*color_state\.r_fix,\s*g_fix:\s*color_state\.g_fix,\s*b_fix:\s*color_state\.b_fix,\s*\}', 'color_state', content)
content = re.sub(r'GouraudSpanStep\s*\{\s*dr_dx:\s*color_step\.dr_dx,\s*dg_dx:\s*color_step\.dg_dx,\s*db_dx:\s*color_step\.db_dx,\s*\}', 'color_step', content)


with open(file_path, "w") as f:
    f.write(content)
