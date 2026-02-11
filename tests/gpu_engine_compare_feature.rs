#![cfg(feature = "gpu-engine-compare")]

#[test]
fn bevy_and_fyrox_dependencies_are_available() {
    let _ = core::any::type_name::<bevy::app::App>();
    let _ = core::any::type_name::<fyrox::engine::Engine>();
}
