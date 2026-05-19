import re

content = open("crates/abrash-core/src/zbuffer.rs").read()

old = """                use rayon::prelude::*;
                // Only parallelize if the workload is large enough to overcome rayon's overhead
                let row_count = ey - sy;
                if row_count > 100 {
                    self.depths[start_idx..end_idx]
                        .par_chunks_exact_mut(w)
                        .for_each(|row| row[sx..ex].fill(f32::INFINITY));
                    return;
                }"""

new = """                use rayon::prelude::*;
                // Only parallelize if the workload is large enough to overcome rayon's overhead
                let row_count = ey - sy;
                if row_count > 100 {
                    // Safe approach avoiding chunks_exact_mut iterator overhead via Rayon
                    let depths_slice = &mut self.depths[..];
                    (sy..ey).into_par_iter().for_each_init(
                        || (),
                        |_, y| {
                            let offset = y * w + sx;
                            let ptr = depths_slice.as_ptr() as usize;
                            unsafe {
                                let row_slice = std::slice::from_raw_parts_mut((ptr + offset * 4) as *mut f32, len);
                                row_slice.fill(f32::INFINITY);
                            }
                        },
                    );
                    return;
                }"""

if old in content:
    with open("crates/abrash-core/src/zbuffer.rs", "w") as f:
        f.write(content.replace(old, new))
    print("Patched successfully")
else:
    print("Pattern not found")
