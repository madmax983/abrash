import re

def main():
    gbuffer_file = "crates/abrash-gpu-render/src/gbuffer.rs"
    with open(gbuffer_file, "r") as f:
        content = f.read()

    content = content.replace(
        "pub const NORMAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;",
        "/// High-precision format used to store XYZ normal vectors and a depth/roughness value.\npub const NORMAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;"
    )
    content = content.replace(
        "pub const ALBEDO_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;",
        "/// HDR-capable format storing the base color of the geometry before lighting is applied.\npub const ALBEDO_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;"
    )
    content = content.replace(
        "pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;",
        "/// Standard depth format ensuring geometry occlusion is handled correctly.\npub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;"
    )

    with open(gbuffer_file, "w") as f:
        f.write(content)

    lib_file = "crates/abrash-gpu-render/src/lib.rs"
    with open(lib_file, "r") as f:
        content = f.read()

    content = content.replace(
        "    IndexCountNotMultipleOf3 { index_count: usize },",
        "    /// Thrown when rendering triangles, but the total number of indices provided cannot be divided into groups of three.\n    IndexCountNotMultipleOf3 {\n        /// The total number of indices that was provided.\n        index_count: usize,\n    },"
    )
    content = content.replace(
        "    IndexOutOfBounds { index: u16, vertex_count: usize },",
        "    /// Thrown when an index in the mesh's index buffer points to a vertex that does not exist.\n    IndexOutOfBounds {\n        /// The invalid index that was encountered.\n        index: u16,\n        /// The total number of vertices actually available.\n        vertex_count: usize,\n    },"
    )
    with open(lib_file, "w") as f:
        f.write(content)

if __name__ == '__main__':
    main()
