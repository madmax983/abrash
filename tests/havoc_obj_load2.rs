use abrash::obj_loader::load_obj;

#[test]
fn test_obj_load_dos_2() {
    let mut bad_obj = String::from("f ");
    for _ in 0..10000 {
        bad_obj.push_str("1/1/1 ");
    }
    // Shouldn't panic. Should probably return error since it exceeds MAX_FACES.
    let _ = load_obj(&bad_obj);
}
