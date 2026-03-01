use abrash::obj_loader::load_obj;

#[test]
fn test_obj_load_dos_4() {
    let mut bad_obj = String::from("");
    for _ in 0..1000000 {
        bad_obj.push_str("v 1 1 1\n");
    }
    // MAX_VERTICES is 1_000_000
    // So if we push exactly 1000000, it should fail or pass cleanly without large allocations?
    let _ = load_obj(&bad_obj);
}
