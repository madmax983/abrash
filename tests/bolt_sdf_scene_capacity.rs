#[cfg(feature = "nova")]
use abrash_render::experimental::sdf::SdfScene;

#[cfg(feature = "nova")]
#[test]
fn test_sdf_scene_with_capacity_allocates_correctly() {
    let scene = SdfScene::with_capacity(456);

    assert_eq!(scene.objects.capacity(), 456);
    assert!(scene.objects.is_empty());
}
