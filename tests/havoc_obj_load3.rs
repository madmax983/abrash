use abrash::obj_loader::load_obj;

#[test]
fn test_obj_load_dos_3() {
    let mut bad_obj = String::from("");
    for i in 1..=50000 {
        bad_obj.push_str(&format!("v {} 0 0\n", i));
    }
    bad_obj.push_str("f ");
    for i in 1..=50000 {
        bad_obj.push_str(&format!("{} ", i));
    }
    // Deep triangulation test
    let _ = load_obj(&bad_obj);
}
