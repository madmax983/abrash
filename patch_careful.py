import os
import re

files = [
    "crates/abrash-render/src/ascii.rs",
    "crates/abrash-render/src/experimental/chroma_key.rs",
    "crates/abrash-render/src/experimental/tilt_shift.rs",
    "crates/abrash-render/src/experimental/pixel_sort.rs",
    "crates/abrash-render/src/experimental/radial_blur.rs",
    "crates/abrash-render/src/experimental/mode7.rs",
]

for filepath in files:
    with open(filepath, "r") as f:
        content = f.read()

    # Apply precise patches only to code (not comments)
    if "ascii.rs" in filepath:
        content = content.replace("chunks_exact(width as usize)", "chunks_exact((width as usize).max(1))")

    if "chroma_key.rs" in filepath:
        content = content.replace("par_chunks_exact(bg_stride)", "par_chunks_exact(bg_stride.max(1))")
        content = content.replace("par_chunks_exact_mut(fg_stride)", "par_chunks_exact_mut(fg_stride.max(1))")

    if "tilt_shift.rs" in filepath:
        content = content.replace("par_chunks_exact_mut(width)", "par_chunks_exact_mut(width.max(1))")
        content = content.replace("par_chunks_exact(width)", "par_chunks_exact(width.max(1))")
        content = content.replace("chunks_exact_mut(width)", "chunks_exact_mut(width.max(1))")
        content = content.replace("chunks_exact(width)", "chunks_exact(width.max(1))")

    if "pixel_sort.rs" in filepath:
        content = re.sub(r'par_chunks_exact_mut\(width\)', r'par_chunks_exact_mut(width.max(1))', content)
        content = re.sub(r'chunks_exact_mut\(width\)', r'chunks_exact_mut(width.max(1))', content)

    if "radial_blur.rs" in filepath:
        content = content.replace("par_chunks_exact_mut(width)", "par_chunks_exact_mut(width.max(1))")
        content = content.replace("chunks_exact_mut(width)", "chunks_exact_mut(width.max(1))")

    if "mode7.rs" in filepath:
        content = content.replace("par_chunks_exact_mut(w)", "par_chunks_exact_mut(w.max(1))")
        content = content.replace("chunks_exact_mut(w)", "chunks_exact_mut(w.max(1))")

    with open(filepath, "w") as f:
        f.write(content)
