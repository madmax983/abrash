#![cfg(feature = "nova")]
use abrash_render::experimental::arboretum::LSystem;

#[test]
fn test_havoc_arboretum_oom_segments() {
    let mut lsys = LSystem::new("F", 90.0, 1.0, 0.1);
    lsys.add_rule('F', "FFFFFFFFFF"); // 10x growth per iteration

    // 8 iterations: 10^8 commands = 100M segments.
    // Each segment pushes 8 vertices. 100M * 8 = 800M vertices.
    // 800M * 12 bytes = 9.6GB.
    let result = lsys.generate_mesh(8);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "L-system exceeded maximum generated segments");
}
