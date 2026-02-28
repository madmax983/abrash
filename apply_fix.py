import re

with open("src/rasterizer/tile.rs", "r") as f:
    content = f.read()

# Pattern 1
pattern1 = r"""\.fold\(Vec::new, \|mut acc, &\[i0, i1, i2\]\| \{
                    // Safety: We trust the indices are within bounds of the vertices slice\.
                    // The caller must ensure this or it will panic inside the thread\.
                    let v0 = vertices\[i0\];
                    let v1 = vertices\[i1\];
                    let v2 = vertices\[i2\];

                    let tris = Self::prepare_triangle_static\(
                        v0,
                        v1,
                        v2,
                        color,
                        width,
                        height,
                        half_width,
                        half_height,
                    \);
                    acc\.extend\(tris\);
                    acc
                \}\)
                \.flatten\(\)"""
replacement1 = r""".flat_map_iter(|&[i0, i1, i2]| {
                    // Safety: We trust the indices are within bounds of the vertices slice.
                    // The caller must ensure this or it will panic inside the thread.
                    let v0 = vertices[i0];
                    let v1 = vertices[i1];
                    let v2 = vertices[i2];

                    Self::prepare_triangle_static(
                        v0,
                        v1,
                        v2,
                        color,
                        width,
                        height,
                        half_width,
                        half_height,
                    )
                })"""

content = re.sub(pattern1, replacement1, content)


# Pattern 2
pattern2 = r"""\.fold\(Vec::new, \|mut acc, &\(([^|]+)\)\| \{
                    let tris = Self::prepare_triangle_static\(
                        v0,
                        v1,
                        v2,
                        color,
                        width,
                        height,
                        half_width,
                        half_height,
                    \);
                    acc\.extend\(tris\);
                    acc
                \}\)
                \.flatten\(\)"""
replacement2 = r""".flat_map_iter(|&(\1)| {
                    Self::prepare_triangle_static(
                        v0,
                        v1,
                        v2,
                        color,
                        width,
                        height,
                        half_width,
                        half_height,
                    )
                })"""

content = re.sub(pattern2, replacement2, content)


# Pattern 3
pattern3 = r"""\.fold\(Vec::new, \|mut acc, &\(([^|]+)\)\| \{
                    let tris = Self::prepare_triangle_textured_static\(
                        \(v0, uv0\),
                        \(v1, uv1\),
                        \(v2, uv2\),
                        tex_w,
                        tex_h,
                        width,
                        height,
                        half_width,
                        half_height,
                    \);
                    acc\.extend\(tris\);
                    acc
                \}\)
                \.flatten\(\)"""
replacement3 = r""".flat_map_iter(|&(\1)| {
                    Self::prepare_triangle_textured_static(
                        (v0, uv0),
                        (v1, uv1),
                        (v2, uv2),
                        tex_w,
                        tex_h,
                        width,
                        height,
                        half_width,
                        half_height,
                    )
                })"""

content = re.sub(pattern3, replacement3, content)


# Pattern 4
pattern4 = r"""\.fold\(Vec::new, \|mut acc, &\(([^|]+)\)\| \{
                    let tris = Self::prepare_triangle_gouraud_static\(
                        v0,
                        v1,
                        v2,
                        width,
                        height,
                        half_width,
                        half_height,
                    \);
                    acc\.extend\(tris\);
                    acc
                \}\)
                \.flatten\(\)"""
replacement4 = r""".flat_map_iter(|&(\1)| {
                    Self::prepare_triangle_gouraud_static(
                        v0,
                        v1,
                        v2,
                        width,
                        height,
                        half_width,
                        half_height,
                    )
                })"""

content = re.sub(pattern4, replacement4, content)

with open("src/rasterizer/tile.rs", "w") as f:
    f.write(content)

