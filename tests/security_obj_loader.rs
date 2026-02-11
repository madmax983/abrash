use abrash::obj_loader::load_obj;

#[test]
fn test_huge_indices_should_fail() {
    let obj_content = "v 0.0 0.0 0.0\n f 1 2 999999999999999";
    let res = load_obj(obj_content);
    assert!(res.is_err());
}

#[test]
fn test_negative_indices_should_fail() {
    let obj_content = "v 0.0 0.0 0.0\n f 1 2 -5";
    let res = load_obj(obj_content);
    assert!(res.is_err());
}

#[test]
fn test_huge_indices_no_panic() {
    let obj_content = "v 0.0 0.0 0.0\n f 9223372036854775807 9223372036854775807 9223372036854775807";
    let res = load_obj(obj_content);
    assert!(res.is_err());
}

#[test]
fn test_non_finite_coordinates_should_fail() {
    let obj_content = "v NaN 0.0 0.0\n f 1 2 3";
    let res = load_obj(obj_content);
    assert!(res.is_err());
}

#[test]
fn test_valid_obj() {
    let obj_content = "v 0.0 0.0 0.0\n v 1.0 0.0 0.0\n v 0.0 1.0 0.0\n f 1 2 3";
    let res = load_obj(obj_content);
    assert!(res.is_ok());
}

#[test]
fn test_vertex_index_zero_should_fail() {
    // OBJ indices are 1-based. 0 is invalid.
    let obj_content = "v 0.0 0.0 0.0\n f 0 1 2";
    let res = load_obj(obj_content);
    assert!(res.is_err());
}

#[test]
fn test_uv_index_zero_should_fail() {
    // OBJ indices are 1-based. 0 is invalid.
    let obj_content = "v 0.0 0.0 0.0\n vt 0.0 0.0\n f 1/0 1/1 1/1";
    let res = load_obj(obj_content);
    assert!(res.is_err());
}
