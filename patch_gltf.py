import re

file_path = "crates/abrash-skeletal/src/gltf_loader.rs"
with open(file_path, "r") as f:
    content = f.read()

replacement = """        let pj = &mut provisional[old_idx];

        let parent = pj.parent_provisional.and_then(|parent_old| {
            let &parent_new = old_to_new.get(&parent_old)?;
            #[allow(clippy::cast_possible_truncation)]
            Some(JointId(parent_new as u16))
        });

        joints.push(Joint {
            // ⚡ Bolt: Use `std::mem::take` to extract the owned String from the provisional
            // struct instead of cloning it, eliding a per-joint heap allocation.
            name: std::mem::take(&mut pj.name),
            parent,
            inverse_bind_matrix: pj.inverse_bind_matrix,
            bind_transform: pj.bind_transform,
        });"""

content = re.sub(
    r"        let pj = &provisional\[old_idx\];\s*let parent = pj\.parent_provisional\.and_then\(\|parent_old\| \{\s*let &parent_new = old_to_new\.get\(&parent_old\)\?;\s*#\[allow\(clippy::cast_possible_truncation\)\]\s*Some\(JointId\(parent_new as u16\)\)\s*\}\);\s*joints\.push\(Joint \{\s*// ⚡ Bolt: Use `std::mem::take` to extract the owned String from the provisional\s*// struct instead of cloning it, eliding a per-joint heap allocation\.\s*name: std::mem::take\(&mut provisional\[old_idx\]\.name\),\s*parent,\s*inverse_bind_matrix: pj\.inverse_bind_matrix,\s*bind_transform: pj\.bind_transform,\s*\}\);",
    replacement,
    content,
    flags=re.MULTILINE
)

with open(file_path, "w") as f:
    f.write(content)
