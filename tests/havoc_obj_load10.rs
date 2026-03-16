use abrash::obj_loader::load_obj;

#[test]
fn test_obj_load_deadlock10() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = load_obj("v 0 0 0\n\nf 1//1 2//2 3//3\n");
    }));
    assert!(result.is_err());
}
