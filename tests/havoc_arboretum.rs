use abrash::experimental::arboretum::LSystem;

#[test]
fn test_memory_limit() {
    // A -> AA (Doubles every iteration)
    let mut lsys = LSystem::new("A", 0.0, 1.0, 1.0);
    lsys.add_rule('A', "AA");

    // 27 iterations = 2^27 = 134,217,728 bytes (~128 MB)
    // The implementation limit is 64 MB.
    // This should now fail gracefully with an Error.
    let res = lsys.expand(27);
    assert!(res.is_err(), "Expected error due to memory limit, got success: {:?}", res.map(|s| s.len()));
}
