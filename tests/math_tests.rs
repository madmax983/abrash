use abrash::math::Vec2;

#[test]
fn test_vec2_new() {
    let v = Vec2::new(3.0, 4.0);
    assert_eq!(v.x, 3.0);
    assert_eq!(v.y, 4.0);
}

#[test]
fn test_vec2_add() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);
    let result = a + b;
    assert_eq!(result, Vec2::new(4.0, 6.0));
}

#[test]
fn test_vec2_sub() {
    let a = Vec2::new(5.0, 7.0);
    let b = Vec2::new(2.0, 3.0);
    let result = a - b;
    assert_eq!(result, Vec2::new(3.0, 4.0));
}

#[test]
fn test_vec2_scale() {
    let v = Vec2::new(2.0, 3.0);
    let result = v * 2.5;
    assert_eq!(result, Vec2::new(5.0, 7.5));
}
