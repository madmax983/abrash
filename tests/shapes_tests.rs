use abrash::shapes::{Polygon, Triangle};
use abrash::math::{Vec2, Mat2};
use std::f32::consts::PI;

#[test]
fn test_polygon_square() {
    let square = Polygon::square(100.0);
    assert_eq!(square.vertices().len(), 4);
}

#[test]
fn test_polygon_regular() {
    let hexagon = Polygon::regular(6, 50.0);
    assert_eq!(hexagon.vertices().len(), 6);
}

#[test]
fn test_polygon_transform() {
    let square = Polygon::square(100.0);
    let rotation = Mat2::rotation(PI / 4.0); // 45 degrees

    let rotated = square.transform(&rotation);

    assert_eq!(rotated.vertices().len(), 4);
    // Vertices should have moved
    assert_ne!(rotated.vertices()[0], square.vertices()[0]);
}

#[test]
fn test_polygon_translate() {
    let square = Polygon::square(100.0);
    let offset = Vec2::new(200.0, 150.0);

    let translated = square.translate(offset);

    // First vertex was at (-50, -50), now should be at (150, 100)
    let v = translated.vertices()[0];
    assert!((v.x - 150.0).abs() < 0.001);
    assert!((v.y - 100.0).abs() < 0.001);
}

#[test]
fn test_polygon_transform_in_place() {
    let mut polygon = Polygon::square(100.0);
    let original_v0 = polygon.vertices()[0];

    let rotation = Mat2::rotation(PI / 4.0);
    polygon.transform_in_place(&rotation);

    // Vertex should have moved
    assert_ne!(polygon.vertices()[0], original_v0);
}

#[test]
fn test_polygon_translate_in_place() {
    let mut polygon = Polygon::square(100.0);
    let offset = Vec2::new(100.0, 100.0);

    polygon.translate_in_place(offset);

    // First vertex was at (-50, -50), now should be at (50, 50)
    let v = polygon.vertices()[0];
    assert!((v.x - 50.0).abs() < 0.001);
    assert!((v.y - 50.0).abs() < 0.001);
}

#[test]
fn test_triangle_new() {
    let tri = Triangle::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 0.0),
        Vec2::new(50.0, 100.0),
    );
    assert_eq!(tri.v0.x, 0.0);
    assert_eq!(tri.v1.x, 100.0);
    assert_eq!(tri.v2.y, 100.0);
}

#[test]
fn test_triangle_translate() {
    let tri = Triangle::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
        Vec2::new(5.0, 10.0),
    );
    let translated = tri.translate(Vec2::new(100.0, 100.0));
    assert!((translated.v0.x - 100.0).abs() < 0.001);
    assert!((translated.v0.y - 100.0).abs() < 0.001);
}
