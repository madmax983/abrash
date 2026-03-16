use abrash::experimental::lsystem::LSystem;

#[test]
fn test_lsystem_dos() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut lsys = LSystem::new("A");
        lsys.add_rule('A', "AA");
        let expanded = lsys.expand(32).unwrap(); // this will generate string of length 2^32, OOM!
    }));
    assert!(result.is_err());
}
