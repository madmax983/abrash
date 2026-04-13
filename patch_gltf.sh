sed -i 's/fn main() {/fn main() -> Result<(), Box<dyn std::error::Error>> {/g' examples/gltf_viewer.rs
sed -i 's/    run_windowed(app);/    run_windowed(app);\n    Ok(())\n}/g' examples/gltf_viewer.rs
