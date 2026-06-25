import re

with open("crates/abrash-core/src/framebuffer.rs", "r") as f:
    content = f.read()

old_code = """
        if sx == 0 && ex == w {
            // Fast path for full-width clears (avoids chunking overhead)
            self.pixels[start_idx..end_idx].fill(color);
        } else {
            let len = ex - sx;
            let mut offset = start_idx + sx;
            let slice = self.pixels.as_mut_slice();
            for _ in sy..ey {
                unsafe {
                    slice.get_unchecked_mut(offset..offset + len).fill(color);
                }
                offset += w;
            }
        }
"""

new_code = """
        if sx == 0 && ex == w {
            // Fast path for full-width clears (avoids chunking overhead)
            self.pixels[start_idx..end_idx].fill(color);
        } else {
            for row in self.pixels[start_idx..end_idx].chunks_exact_mut(w) {
                // SAFETY: We manually clamped sx and ex to width above,
                // and row is exactly width elements long.
                unsafe {
                    row.get_unchecked_mut(sx..ex).fill(color);
                }
            }
        }
"""

content = content.replace(old_code.strip(), new_code.strip())

with open("crates/abrash-core/src/framebuffer.rs", "w") as f:
    f.write(content)
