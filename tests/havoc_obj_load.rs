use abrash::obj_loader::load_obj;

#[test]
fn test_obj_load_dos() {
    // Generate a long string of valid format but deep nesting/many spaces
    let mut bad_obj = String::from("v 1.0 1.0 1.0 ");
    for _ in 0..100000 {
        bad_obj.push(' ');
    }
    // Shouldn't crash or take extremely long
    let _ = load_obj(&bad_obj);
}
