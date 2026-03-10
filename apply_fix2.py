with open('src/rasterizer/tile.rs', 'r') as f:
    content = f.read()

content = content.replace("align_bytes.div_ceil(elem_size)", "(align_bytes + elem_size - 1) / elem_size")
content = content.replace("let align_bytes = 32;", "let align_bytes: usize = 32;")
content = content.replace("let align_bytes: i32 = 32;", "let align_bytes: usize = 32;")

with open('src/rasterizer/tile.rs', 'w') as f:
    f.write(content)

with open('src/rasterizer/texture.rs', 'r') as f:
    t_content = f.read()

import re
t_content = re.sub(r'let u_c = _mm256_min_epi32\(_mm256_max_epi32\(u_i, zero_i\), max_x\);\n\s*let v_c = _mm256_min_epi32\(_mm256_max_epi32\(v_i, zero_i\), max_y\);', '', t_content)

with open('src/rasterizer/texture.rs', 'w') as f:
    f.write(t_content)
