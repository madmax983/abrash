import sys

with open("crates/abrash-render/src/rasterizer/pbr.rs", "r") as f:
    content = f.read()

content = content.replace(
    "let old_pixels = _mm256_loadu_si256(pixels_ptr.cast::<__m256i>());",
    "#[allow(clippy::cast_ptr_alignment)]\n                    let old_pixels = _mm256_loadu_si256(pixels_ptr.cast::<__m256i>());"
)

content = content.replace(
    "pixels_ptr.cast::<__m256i>(),",
    "#[allow(clippy::cast_ptr_alignment)]\n                    pixels_ptr.cast::<__m256i>(),"
)

with open("crates/abrash-render/src/rasterizer/pbr.rs", "w") as f:
    f.write(content)
