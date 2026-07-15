file_path = "crates/abrash-render/src/experimental/tunnel.rs"
with open(file_path, "r") as f:
    content = f.read()

content = content.replace("use abrash_core::math::funcs::fast_atan2;", "use abrash_core::math::fast_atan2;")

with open(file_path, "w") as f:
    f.write(content)
