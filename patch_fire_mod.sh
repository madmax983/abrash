sed -i 's/pub mod fisheye;/pub mod fisheye;\npub mod fire;/g' crates/abrash-render/src/experimental/mod.rs
cargo check --example fire_demo --features nova
