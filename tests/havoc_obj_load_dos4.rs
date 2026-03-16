use abrash::obj_loader::load_obj;

#[test]
fn test_obj_load_dos4() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = load_obj("f 1 2\n");
    }));
    assert!(result.is_err());
}
