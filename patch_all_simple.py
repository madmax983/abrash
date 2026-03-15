import re

def simple_patch(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Apply the proper fix.
    content = content.replace("let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };", "let fb_slice = &mut fb.as_mut_slice()[start_idx..=end_idx];")
    content = content.replace("let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };", "let zb_slice = &mut zb.as_mut_slice()[start_idx..=end_idx];")

    content = content.replace("let fb_slice = fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx);", "let fb_slice = &mut fb.as_mut_slice()[start_idx..=end_idx];")
    content = content.replace("let zb_slice = zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx);", "let zb_slice = &mut zb.as_mut_slice()[start_idx..=end_idx];")

    with open(filepath, 'w') as f:
        f.write(content)

for path in ["src/rasterizer/texture.rs", "src/rasterizer/phong.rs", "src/rasterizer/reflection.rs", "src/rasterizer/gouraud.rs", "src/rasterizer/pbr.rs", "src/rasterizer/core.rs"]:
    simple_patch(path)
