use abrash::obj_loader::load_obj;

#[test]
fn test_valid_obj() {
    let obj = "
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
";
    let mesh = load_obj(obj).unwrap();
    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.indices.len(), 1);
}

#[test]
fn test_huge_indices_should_fail() {
    // Index 4 out of bounds (only 3 vertices)
    let obj = "
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 4
";
    let res = load_obj(obj);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Vertex index 4 out of bounds"));
}

#[test]
fn test_negative_indices_should_fail() {
    // Negative indices are not supported by simple parser
    let obj = "
v 0 0 0
v 1 0 0
v 0 1 0
f -1 -2 -3
";
    let res = load_obj(obj);
    assert!(res.is_err());
    // Error message depends on parse failure or logic
    // Current impl fails at parse::<usize>("-1")
    let err = res.unwrap_err();
    assert!(err.contains("Invalid vertex index") || err.contains("Vertex index 0 is invalid"));
}

#[test]
fn test_vertex_index_zero_should_fail() {
    // OBJ is 1-based
    let obj = "
v 0 0 0
f 0 1 1
";
    let res = load_obj(obj);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Vertex index 0 is invalid"));
}

#[test]
fn test_uv_index_zero_should_fail() {
    let obj = "
v 0 0 0
vt 0 0
f 1/0 1/1 1/1
";
    let res = load_obj(obj);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("UV index 0 is invalid"));
}

#[test]
fn test_non_finite_coordinates_should_fail() {
    let obj = "
v NaN 0 0
";
    let res = load_obj(obj);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Coordinates must be finite"));
}

#[test]
fn test_huge_indices_no_panic() {
    // Extremely large index to test usize overflow handling or OOM
    // Though we can't allocate huge vectors easily, we can check index parsing
    let obj = "
v 0 0 0
f 9999999999999999999999999 1 1
";
    let res = load_obj(obj);
    assert!(res.is_err());
    // Should fail parsing index
    assert!(res.unwrap_err().contains("Invalid vertex index"));
}
