use abrash::mesh::Mesh;

#[test]
fn test_load_obj_simple() {
    let obj_content = r#"
        # This is a comment
        v 0.0 0.0 0.0
        v 1.0 0.0 0.0
        v 0.0 1.0 0.0

        f 1 2 3
    "#;

    let mesh = Mesh::from_obj(obj_content);

    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.indices.len(), 1);

    // Check vertex positions
    assert_eq!(mesh.vertices[0].x, 0.0);
    assert_eq!(mesh.vertices[1].x, 1.0);
    assert_eq!(mesh.vertices[2].y, 1.0);

    // Check indices (0-based in Mesh, 1-based in OBJ)
    assert_eq!(mesh.indices[0], [0, 1, 2]);
}

#[test]
fn test_load_obj_quad() {
    let obj_content = r#"
        v -1.0 -1.0 0.0
        v 1.0 -1.0 0.0
        v 1.0 1.0 0.0
        v -1.0 1.0 0.0

        # Quad should be triangulated
        f 1 2 3 4
    "#;

    let mesh = Mesh::from_obj(obj_content);

    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.indices.len(), 2); // 2 triangles
}

#[test]
fn test_load_obj_with_invalid_indices() {
    // OBJ content with:
    // - Valid vertices (0, 1, 2)
    // - Face 1: Valid (1 2 3) -> indices [0, 1, 2]
    // - Face 2: Out of bounds positive (1 2 999) -> Should be sanitized (ignored)
    // - Face 3: Out of bounds negative (1 2 -999) -> Should be sanitized (ignored)
    let obj_content = r#"
        v 0.0 0.0 0.0
        v 1.0 0.0 0.0
        v 0.0 1.0 0.0

        f 1 2 3
        f 1 2 999
        f 1 2 -999
    "#;

    let mesh = Mesh::from_obj(obj_content);

    // Verify all indices are valid
    for tri in &mesh.indices {
        for &idx in tri.iter() {
            assert!(
                idx < mesh.vertices.len(),
                "Index {} is out of bounds (len: {})",
                idx,
                mesh.vertices.len()
            );
        }
    }

    // Verify that compute_face_normals does not panic
    let _normals = mesh.compute_face_normals();

    // Verify that invalid faces were effectively ignored/triangulation didn't happen for them
    // Face 1: 3 vertices -> 1 triangle
    // Face 2: "1 2" (999 ignored) -> 2 vertices -> 0 triangles
    // Face 3: "1 2" (-999 ignored) -> 2 vertices -> 0 triangles
    assert_eq!(mesh.indices.len(), 1);
}
