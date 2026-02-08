use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=shaders/");

    // Check if GPU binning feature is enabled
    if env::var("CARGO_FEATURE_GPU_BINNING").is_err() {
        return;
    }

    let shaders = vec![
        (
            "shaders/binning.hlsl",
            "shaders/binning.cso",
            "BinTrianglesCS",
        ),
        (
            "shaders/coarse_binning.hlsl",
            "shaders/coarse_binning.cso",
            "CoarseBinningCS",
        ),
        (
            "shaders/fine_binning.hlsl",
            "shaders/fine_binning.cso",
            "FineBinningCS",
        ),
    ];

    let dxc_path = find_dxc();

    if let Some(dxc) = dxc_path {
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
    } else {
        println!("cargo:warning=DXC not found. GPU binning will not work.");
        println!("cargo:warning=Install Windows SDK or place dxc.exe in PATH");
        // Don't fail the build - just warn. The feature will compile but not run.
    }
}

fn find_dxc() -> Option<String> {
    // Try to find dxc in PATH
    if let Ok(output) = std::process::Command::new("dxc").arg("--version").output() {
        if output.status.success() {
            return Some("dxc".to_string());
        }
    }

    // Common Windows SDK locations
    let possible_paths = [
        r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\dxc.exe",
        r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.20348.0\x64\dxc.exe",
        r"C:\Program Files (x86)\Windows Kits\10\bin\x64\dxc.exe",
    ];

    for path in possible_paths {
        if PathBuf::from(path).exists() {
            return Some(path.to_string());
        }
    }

    None
}
