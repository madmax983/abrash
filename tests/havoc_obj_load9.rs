use abrash::obj_loader::load_obj;

#[test]
fn test_obj_load_deadlock9() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = load_obj("v 0 0 0\n\nf 1 1 1\nf 1// 1// 1//\nf 1/1/ 1/1/ 1/1/\n");
    }));
    assert!(result.is_err());
}
