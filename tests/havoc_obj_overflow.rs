use abrash::obj_loader::load_obj;

#[test]
fn test_integer_overflow_parsing() {
    // 20 digits: 20000000000000000000
    // u64::MAX is ~1.84e19 (20 digits, but starts with 1)
    // So this overflows u64.
    // It passes the digit count check (20), so it hits the checked arithmetic.
    // We expect an Error (None from fast_parse_usize -> "Invalid vertex index").
    let bad_obj = "v 0 0 0\nf 20000000000000000000";

    let res = load_obj(bad_obj);
    assert!(res.is_err(), "Should return error on overflow, got {:?}", res);
}
