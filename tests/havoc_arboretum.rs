#![cfg(feature = "nova")]
use abrash_render::experimental::arboretum::LSystem;

#[test]
fn test_arboretum_panic_bytes() {
    let mut lsys = LSystem::new("螃", 90.0, 1.0, 0.1);
    lsys.add_rule('螃', "F螃");
    let result = lsys.expand(2).unwrap();
    println!("result: {result:?}");
    assert_eq!(result, "FF螃");
}
