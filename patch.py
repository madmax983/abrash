import re

file_path = "crates/abrash-render/src/rasterizer/texture.rs"

with open(file_path, "r") as f:
    content = f.read()

content = re.sub(
    r"let max_x = _mm256_set1_epi32\(\(texture\.width - 1\) as i32\);",
    r"let max_x = _mm256_set1_epi32((texture.width.saturating_sub(2).max(0)) as i32);",
    content
)

content = re.sub(
    r"let max_y = _mm256_set1_epi32\(\(texture\.height - 1\) as i32\);",
    r"let max_y = _mm256_set1_epi32((texture.height.saturating_sub(2).max(0)) as i32);",
    content
)

content = re.sub(
    r"let max_x = _mm256_set1_epi32\(\(normal_map\.width - 1\) as i32\);",
    r"let max_x = _mm256_set1_epi32((normal_map.width.saturating_sub(2).max(0)) as i32);",
    content
)

content = re.sub(
    r"let max_y = _mm256_set1_epi32\(\(normal_map\.height - 1\) as i32\);",
    r"let max_y = _mm256_set1_epi32((normal_map.height.saturating_sub(2).max(0)) as i32);",
    content
)

with open(file_path, "w") as f:
    f.write(content)

print("Patched texture.rs")
