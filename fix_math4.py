import re

with open("crates/abrash-core/src/math.rs", "r") as f:
    content = f.read()

content = content.replace("0x030000ff", "0x0300_00ff")
content = content.replace("0x0300f00f", "0x0300_f00f")
content = content.replace("0x030c30c3", "0x030c_30c3")
content = content.replace("0x09249249", "0x0924_9249")
content = content.replace("0 => dx + dy,\n            1 => -dx + dy,", "0 | 12 => dx + dy,\n            1 | 13 => -dx + dy,")
content = content.replace("9 => -dy + dz,\n            10 => dy - dz,\n            11 => -dy - dz,\n            12 => dx + dy,\n            13 => -dx + dy,\n            14 => -dy + dz,", "9 | 14 => -dy + dz,\n            10 => dy - dz,")

content = content.replace("/// Test whether an AABB (`min`, `max`) intersects the frustum. Uses the\n/// p-vertex (positive-vertex) test: for each plane the \"most positive\" corner\n/// is tested; if that corner is outside the plane the AABB is fully outside.", "/// Test whether an AABB (`min`, `max`) intersects the frustum.\n///\n/// Uses the p-vertex (positive-vertex) test: for each plane the \"most positive\" corner\n/// is tested; if that corner is outside the plane the AABB is fully outside.")

content = content.replace("    let x = if t < 4000.0 {\n        let ti = 1.0 / t;\n        -0.266_123_9e9 * ti * ti * ti - 0.234_358_0e6 * ti * ti + 0.877_695_6e3 * ti + 0.179_910\n    } else {\n        let ti = 1.0 / t;\n        -3.025_846_9e9 * ti * ti * ti + 2.107_037_9e6 * ti * ti + 0.222_634_7e3 * ti + 0.240_390\n    };", "    let ti = 1.0 / t;\n    let x = if t < 4000.0 {\n        -0.266_123_9e9 * ti * ti * ti - 0.234_358_0e6 * ti * ti + 0.877_695_6e3 * ti + 0.179_910\n    } else {\n        -3.025_846_9e9 * ti * ti * ti + 2.107_037_9e6 * ti * ti + 0.222_634_7e3 * ti + 0.240_390\n    };")


with open("crates/abrash-core/src/math.rs", "w") as f:
    f.write(content)

with open("crates/abrash-core/src/sdf.rs", "r") as f:
    content = f.read()

content = content.replace("    let c1 = (ey.x * ap.x + ey.y * ap.y) + (ab.x * ab.x + ab.y * ab.y);", "    #[allow(clippy::suspicious_operation_groupings)]\n    let c1 = (ey.x * ap.x + ey.y * ap.y) + (ab.x * ab.x + ab.y * ab.y);")

with open("crates/abrash-core/src/sdf.rs", "w") as f:
    f.write(content)


with open("crates/abrash-core/src/geometry.rs", "r") as f:
    content = f.read()

content = content.replace("    pub fn from_radius_height(center: Vec3, radius: f32, height: f32) -> Self {", "    pub const fn from_radius_height(center: Vec3, radius: f32, height: f32) -> Self {")

with open("crates/abrash-core/src/geometry.rs", "w") as f:
    f.write(content)
