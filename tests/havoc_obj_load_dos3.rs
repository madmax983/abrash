use abrash::obj_loader::load_obj;

#[test]
fn test_obj_load_dos3() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = load_obj("f -1 -1 -1\n");
    }));
    assert!(result.is_err());
}
