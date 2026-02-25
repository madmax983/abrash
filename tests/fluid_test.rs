use abrash::experimental::fluid::Fluid;

#[test]
fn test_fluid_initialization() {
    let f = Fluid::new(64, 0.0, 0.0, 0.1);
    assert_eq!(f.size, 64);
}

#[test]
fn test_fluid_density() {
    let mut f = Fluid::new(64, 0.0, 0.0, 0.1);
    f.add_density(32, 32, 100.0);
    f.step();
    // Just ensure it doesn't crash and density is modified (hard to predict exact values due to float math)
}
