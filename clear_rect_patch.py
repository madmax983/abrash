import re

def patch_file(file_path, old_code, new_code):
    with open(file_path, 'r') as f:
        content = f.read()
    content = content.replace(old_code, new_code)
    with open(file_path, 'w') as f:
        f.write(content)

old_fb = """            let len = ex - sx;
            let mut offset = start_idx + sx;
            let slice = self.pixels.as_mut_slice();
            for _ in sy..ey {
                unsafe {
                    slice.get_unchecked_mut(offset..offset + len).fill(color);
                }
                offset += w;
            }"""

new_fb = """            self.pixels[start_idx..end_idx]
                .chunks_exact_mut(w)
                .for_each(|row| {
                    row[sx..ex].fill(color);
                });"""

patch_file('crates/abrash-core/src/framebuffer.rs', old_fb, new_fb)

old_zb = """            let mut offset = start_idx + sx;
            let slice = self.depths.as_mut_slice();
            for _ in sy..ey {
                unsafe {
                    slice
                        .get_unchecked_mut(offset..offset + len)
                        .fill(f32::INFINITY);
                }
                offset += w;
            }"""

new_zb = """            self.depths[start_idx..end_idx]
                .chunks_exact_mut(w)
                .for_each(|row| {
                    row[sx..ex].fill(f32::INFINITY);
                });"""

patch_file('crates/abrash-core/src/zbuffer.rs', old_zb, new_zb)
