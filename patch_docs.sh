# Fix backslashes introduced by sed
sed -i 's/\\\/\/\//\/\/\//g' crates/abrash-core/src/math/vec2.rs
sed -i 's/\\\/\/\//\/\/\//g' crates/abrash-core/src/math/vec3.rs
sed -i 's/\\\/\/\//\/\/\//g' crates/abrash-core/src/math/vec4.rs
sed -i 's/\\\/\/\//\/\/\//g' crates/abrash-core/src/quat.rs
sed -i 's/\\\/\/\//\/\/\//g' crates/abrash-core/src/math/mat3.rs
sed -i 's/\\\/\/\//\/\/\//g' crates/abrash-core/src/math/mat4.rs
sed -i 's/\\\/\/\//\/\/\//g' crates/abrash-core/src/geometry.rs
sed -i 's/\\\/\/\//\/\/\//g' crates/abrash-core/src/transform.rs
sed -i 's/\\\/\/\//\/\/\//g' crates/abrash-core/src/texture.rs
sed -i 's/\\\/\/\//\/\/\//g' crates/abrash-core/src/math/screen_point.rs
