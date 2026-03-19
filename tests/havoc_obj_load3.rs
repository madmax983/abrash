use abrash::obj_loader::load_obj;
use std::fmt::Write as _;

#[test]
fn test_obj_load_dos_3() {
    let mut bad_obj = String::new();
    for i in 1..=50000 {
        writeln!(bad_obj, "v {i} 0 0").expect("writing to String cannot fail");
    }
    bad_obj.push_str("f ");
    for i in 1..=50000 {
        write!(bad_obj, "{i} ").expect("writing to String cannot fail");
    }
    // Deep triangulation test
    let _ = load_obj(&bad_obj);
}
