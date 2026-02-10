use abrash::gpu_render::{mesh_to_gpu, run_mesh_demo, GpuDemoConfig};
use abrash::obj_loader::load_obj;

const SPACESHIP_OBJ: &str = r#"
# Simple Spacerocket
v 0.0 1.5 0.0
v 0.5 -0.5 0.5
v -0.5 -0.5 0.5
v -0.5 -0.5 -0.5
v 0.5 -0.5 -0.5
v 0.0 -0.8 0.0
# Top pyramid
f 1 2 3
f 1 3 4
f 1 4 5
f 1 5 2
# Bottom inverted pyramid (engine)
f 6 3 2
f 6 4 3
f 6 5 4
f 6 2 5
"#;

fn main() -> Result<(), String> {
    // 1. Load Mesh (CPU)
    println!("Loading OBJ...");
    let mesh = load_obj(SPACESHIP_OBJ).map_err(|e| e.to_string())?;
    println!("Mesh loaded: {} vertices, {} triangles", mesh.vertices.len(), mesh.indices.len());

    // 2. Convert to GPU Format (Bridge)
    println!("Converting to GPU format...");
    let (vertices, indices) = mesh_to_gpu(&mesh)?;
    println!("Conversion successful: {} GPU vertices, {} indices", vertices.len(), indices.len());

    // 3. Configure and Run (GPU)
    let config = GpuDemoConfig {
        title: "Abrash GPU OBJ Viewer".to_string(),
        initial_distance: 3.0,
        ..GpuDemoConfig::default()
    };

    println!("Starting GPU demo...");
    run_mesh_demo(vertices, indices, config)
}
