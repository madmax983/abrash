use std::fs;
fn main() {
    let mut s = fs::read_to_string("crates/abrash-render/src/render_api/cpu_renderer.rs").unwrap();
    s = s.replace(
r#"        if cpu_mesh.mesh.indices.len() != mesh.indices.len() || cpu_mesh.mesh.indices != mesh.indices {
            cpu_mesh.shared_indices = std::sync::Arc::from(mesh.indices.as_slice());
            cpu_mesh.mesh.indices.clone_from(&mesh.indices);
        }
        cpu_mesh.mesh.vertices.clone_from(&mesh.vertices);
        cpu_mesh.mesh.uvs.clone_from(&mesh.uvs);
        cpu_mesh.mesh.normals.clone_from(&mesh.normals);
        cpu_mesh.mesh.tangents.clone_from(&mesh.tangents);"#,
r#"        // Avoid re-allocating the shared index Arc and copying indices if they haven't changed.
        // This avoids allocations on hot-paths like skeletal animation where only vertices move.
        if cpu_mesh.mesh.indices.len() != mesh.indices.len() || cpu_mesh.mesh.indices != mesh.indices {
            cpu_mesh.shared_indices = std::sync::Arc::from(mesh.indices.as_slice());
            cpu_mesh.mesh.indices.clone_from(&mesh.indices);
        }

        cpu_mesh.mesh.vertices.clone_from(&mesh.vertices);
        cpu_mesh.mesh.uvs.clone_from(&mesh.uvs);
        cpu_mesh.mesh.normals.clone_from(&mesh.normals);
        cpu_mesh.mesh.tangents.clone_from(&mesh.tangents);"#,
);
    fs::write("crates/abrash-render/src/render_api/cpu_renderer.rs", s).unwrap();
}
