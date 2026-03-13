use std::fmt::Write;
use abrash::obj_loader::load_obj;

#[test]
fn test_obj_load_dos_3() {
    let mut bad_obj = String::new();
    for i in 1..=50000 {
        let _ = writeln!(bad_obj, "v {i} 0 0");
    }
    bad_obj.push_str("f ");
    for i in 1..=50000 {
        let _ = write!(bad_obj, "{i} ");
    }
    // Deep triangulation test
    let _ = load_obj(&bad_obj);
}
