use abrash::obj_loader::load_obj;
use proptest::prelude::*;

#[test]
#[ignore = "👺 Havoc: Intentionally triggers OOM or massive allocation."]
fn test_obj_loader_dos() {
    let mut bad_obj = String::with_capacity(300_000_000);
    bad_obj.push_str("v 1.0 1.0 1.0\n");
    for _ in 0..1_000_000 {
        bad_obj.push_str("vt 0.0 0.0\n");
    }

    bad_obj.push_str("f ");
    for _ in 0..10_000_000 {
        bad_obj.push_str("1/1 ");
    }
    bad_obj.push('\n');
    let _ = load_obj(&bad_obj);
}
