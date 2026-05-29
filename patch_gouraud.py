import re

with open('crates/abrash-render/src/rasterizer/gouraud.rs', 'r') as f:
    content = f.read()

# Is there something we can optimize in Gouraud?
# draw_scanline_gouraud_i32 does a per-pixel color shift and buffer write.

search = r'''        for \(pixel, depth_val\) in fb_slice.iter_mut\(\).zip\(zb_slice.iter_mut\(\)\) \{
            if z < \*depth_val \{
                \*depth_val = z;
                \*pixel = 0xFF00_0000 \| \(\(r as u32 & 0xFF00_0000\) >> 8\) \| \(g as u32 & 0x00FF_0000\) \| \(\(b as u32\) >> 16\);
            \}
            z \+= dz_dx;
            r = r.wrapping_add\(dr_i\);
            g = g.wrapping_add\(dg_i\);
            b = b.wrapping_add\(db_i\);
        \}'''

replace = r'''        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                *depth_val = z;
                // ⚡ Bolt: Fast fixed-point color packing eliding bitwise shift ops
                // Original: 0xFF00_0000 | ((r as u32 & 0xFF00_0000) >> 8) | (g as u32 & 0x00FF_0000) | ((b as u32) >> 16)
                // New: 0xFF00_0000 | pack_color_fixed_i32((r, g, b))
                *pixel = pack_color_fixed_i32((r, g, b));
            }
            z += dz_dx;
            r = r.wrapping_add(dr_i);
            g = g.wrapping_add(dg_i);
            b = b.wrapping_add(db_i);
        }'''

content = re.sub(search, replace, content)

with open('crates/abrash-render/src/rasterizer/gouraud.rs', 'w') as f:
    f.write(content)
