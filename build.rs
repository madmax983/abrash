// Build script for compiling HLSL shaders to DXIL bytecode
// Only runs when gpu-binning feature is enabled

fn main() {
    #[cfg(feature = "gpu-binning")]
    compile_shaders();
}

#[cfg(feature = "gpu-binning")]
fn compile_shaders() {
    use std::process::Command;

    // List of shaders to compile: (source, output, entry_point)
    let shaders = [
        (
            "shaders/bin_triangles.hlsl",
            "shaders/bin_triangles.cso",
            "BinTriangles",
        ),
        (
            "shaders/bin_coarse.hlsl",
            "shaders/bin_coarse.cso",
            "BinCoarse",
        ),
        ("shaders/bin_fine.hlsl", "shaders/bin_fine.cso", "BinFine"),
        (
            "shaders/build_hiz_level.hlsl",
            "shaders/build_hiz_level.cso",
            "BuildHiZLevel",
        ),
    ];

    // Check if DXC (DirectX Shader Compiler) is available
    let dxc_path = find_dxc();

    match dxc_path {
        Some(dxc) => {
            for (shader_src, shader_out, entry_point) in &shaders {
                println!("cargo:rerun-if-changed={shader_src}");
                println!("cargo:info=Compiling shader: {shader_src} -> {shader_out}");

                let output = Command::new(&dxc)
                    .arg("-T")
                    .arg("cs_6_0") // Compute Shader Model 6.0
                    .arg("-E")
                    .arg(entry_point) // Entry point
                    .arg("-Fo")
                    .arg(shader_out)
                    .arg(shader_src)
                    .arg("-Zi") // Debug info
                    .arg("-Qembed_debug") // Embed debug info in shader
                    .output()
                    .expect("Failed to execute dxc.exe");

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    panic!("Shader compilation failed for {shader_src}:\n{stderr}");
                }

                println!("cargo:info=Shader {shader_src} compilation successful");
            }
        }
        None => {
            println!("cargo:warning=DXC not found. GPU binning will not work.");
            println!("cargo:warning=Install Windows SDK or place dxc.exe in PATH");
            // Don't fail the build - just warn. The feature will compile but not run.
        }
    }
}

#[cfg(feature = "gpu-binning")]
fn find_dxc() -> Option<String> {
    // Try to find dxc.exe in common locations

    // 1. Check if it's in PATH
    if let Ok(output) = std::process::Command::new("dxc").arg("--version").output() {
        if output.status.success() {
            return Some("dxc".to_string());
        }
    }

    // 2. Check Windows SDK locations
    let sdk_paths = [
        r"C:\Program Files (x86)\Windows Kits\10\bin\x64\dxc.exe",
        r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\dxc.exe",
        r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.22000.0\x64\dxc.exe",
        r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64\dxc.exe",
    ];

    for path in &sdk_paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    None
}
