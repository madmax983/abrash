use abrash::obj_loader::load_obj;

#[test]
fn test_obj_load_deadlock7() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = load_obj("v 0 0 0\nf -1//-1 -1//-1 -1//-1\n"); // Out of bounds vertex index
    }));
    assert!(result.is_err());
}
