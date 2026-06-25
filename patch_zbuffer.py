import re

with open("crates/abrash-core/src/zbuffer.rs", "r") as f:
    content = f.read()

old_code = """
        if sx == 0 && ex == w {
            self.depths[start_idx..end_idx].fill(f32::INFINITY);
        } else {
            let len = ex - sx;

            // Hot path optimization: process rows concurrently if large enough
            #[cfg(feature = "parallel")]
            {
                use rayon::prelude::*;
                // Only parallelize if the workload is large enough to overcome rayon's overhead
                let row_count = ey - sy;
                if row_count > 100 {
                    self.depths[start_idx..end_idx]
                        .par_chunks_exact_mut(w)
                        .for_each(|row| row[sx..ex].fill(f32::INFINITY));
                    return;
                }
            }

            let mut offset = start_idx + sx;
            let slice = self.depths.as_mut_slice();
            for _ in sy..ey {
                unsafe {
                    slice
                        .get_unchecked_mut(offset..offset + len)
                        .fill(f32::INFINITY);
                }
                offset += w;
            }
        }
"""

new_code = """
        if sx == 0 && ex == w {
            self.depths[start_idx..end_idx].fill(f32::INFINITY);
        } else {
            // Hot path optimization: process rows concurrently if large enough
            #[cfg(feature = "parallel")]
            {
                use rayon::prelude::*;
                // Only parallelize if the workload is large enough to overcome rayon's overhead
                let row_count = ey - sy;
                if row_count > 100 {
                    self.depths[start_idx..end_idx]
                        .par_chunks_exact_mut(w)
                        .for_each(|row| {
                            // SAFETY: We clamped sx and ex to width above, and row is exactly `width` elements long.
                            unsafe {
                                row.get_unchecked_mut(sx..ex).fill(f32::INFINITY);
                            }
                        });
                    return;
                }
            }

            for row in self.depths[start_idx..end_idx].chunks_exact_mut(w) {
                // SAFETY: We clamped sx and ex to width above, and row is exactly `width` elements long.
                unsafe {
                    row.get_unchecked_mut(sx..ex).fill(f32::INFINITY);
                }
            }
        }
"""

content = content.replace(old_code.strip(), new_code.strip())

with open("crates/abrash-core/src/zbuffer.rs", "w") as f:
    f.write(content)
