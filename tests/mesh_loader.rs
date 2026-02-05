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
