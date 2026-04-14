sed -i 's/pub mod ascii;/pub mod ascii;\npub mod fire;/g' crates/abrash-render/src/experimental/mod.rs
cargo check --example fire_demo --features nova
