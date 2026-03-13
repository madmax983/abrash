use abrash::obj_loader::load_obj;
use std::fmt::Write;

#[test]
fn test_obj_load_dos_3() {
    let mut bad_obj = String::new();
    for i in 1..=50000 {
        use std::fmt::Write;
        let _ = write!(bad_obj, "v {i} 0 0\n");
    }
    bad_obj.push_str("f ");
    for i in 1..=50000 {
        let _ = write!(bad_obj, "{i} ");
    }
    // Deep triangulation test
    let _ = load_obj(&bad_obj);
}
