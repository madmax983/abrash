//! Acceleration structure management for hardware ray tracing.
//!
//! Builds Bottom-Level Acceleration Structures (BLAS) from mesh vertex/index data
//! and Top-Level Acceleration Structures (TLAS) from per-instance transforms.
//!
//! Requires `wgpu::Features::EXPERIMENTAL_RAY_QUERY` (Vulkan backend only).

use abrash_core::math::Mat4;

/// A built BLAS for a single mesh.
pub struct MeshBlas {
    pub(crate) blas: wgpu::Blas,
}

impl MeshBlas {
    /// Build a BLAS from a mesh's vertex and index buffers.
    ///
    /// The vertex buffer must contain positions at offset 0 with the given stride.
    #[must_use]
    pub fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertex_buffer: &wgpu::Buffer,
        vertex_count: u32,
        vertex_stride: u64,
        index_buffer: &wgpu::Buffer,
        index_count: u32,
    ) -> Self {
        let size_desc = wgpu::BlasTriangleGeometrySizeDescriptor {
            vertex_format: wgpu::VertexFormat::Float32x3,
            vertex_count,
            index_format: Some(wgpu::IndexFormat::Uint32),
            index_count: Some(index_count),
            flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
        };

        let blas = device.create_blas(
            &wgpu::CreateBlasDescriptor {
                label: Some("Mesh BLAS"),
                flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                update_mode: wgpu::AccelerationStructureUpdateMode::Build,
            },
            wgpu::BlasGeometrySizeDescriptors::Triangles {
                descriptors: vec![size_desc.clone()],
            },
        );

        let geometry = wgpu::BlasTriangleGeometry {
            size: &size_desc,
            vertex_buffer,
            first_vertex: 0,
            vertex_stride,
            index_buffer: Some(index_buffer),
            first_index: Some(0),
            transform_buffer: None,
            transform_buffer_offset: None,
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("BLAS Build Encoder"),
        });
        encoder.build_acceleration_structures(
            std::iter::once(&wgpu::BlasBuildEntry {
                blas: &blas,
                geometry: wgpu::BlasGeometries::TriangleGeometries(vec![geometry]),
            }),
            std::iter::empty(),
        );
        queue.submit(Some(encoder.finish()));

        Self { blas }
    }
}

/// Top-Level Acceleration Structure built from mesh instances with transforms.
pub struct SceneTlas {
    pub(crate) tlas: wgpu::Tlas,
}

impl SceneTlas {
    /// Build a TLAS from a list of (BLAS, transform) pairs.
    ///
    /// Each instance gets a 3×4 affine transform matrix derived from the Mat4.
    #[must_use]
    pub fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[(&MeshBlas, &Mat4)],
    ) -> Self {
        let max_instances = instances.len().max(1) as u32;

        let mut tlas = device.create_tlas(&wgpu::CreateTlasDescriptor {
            label: Some("Scene TLAS"),
            max_instances,
            flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
            update_mode: wgpu::AccelerationStructureUpdateMode::Build,
        });

        // Fill instances
        for (i, (mesh_blas, transform)) in instances.iter().enumerate() {
            // Convert Mat4 (4×4 row-major, row-vector) to 3×4 affine for RT.
            // RT expects column-vector convention where translation is in column 3.
            // Our row-vector Mat4 has translation in row 3, so we transpose.
            let m = &transform.m;
            let affine: [f32; 12] = [
                m[0][0], m[1][0], m[2][0], m[3][0], // transposed row 0
                m[0][1], m[1][1], m[2][1], m[3][1], // transposed row 1
                m[0][2], m[1][2], m[2][2], m[3][2], // transposed row 2
            ];

            tlas[i] = Some(wgpu::TlasInstance::new(
                &mesh_blas.blas,
                affine,
                i as u32, // custom_data = instance index
                0xFF,     // mask = all bits (visible to all ray masks)
            ));
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("TLAS Build Encoder"),
        });
        encoder.build_acceleration_structures(std::iter::empty(), std::iter::once(&tlas));
        queue.submit(Some(encoder.finish()));

        Self { tlas }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_affine_transform_translation() {
        use abrash_core::math::Mat4;
        let m = Mat4::translation(1.0, 2.0, 3.0);
        // Row-vector convention: translation in row 3 → m[3] = [tx, ty, tz, 1]
        // Transposed for RT: translation ends up in column 3 of each row
        let affine: [f32; 12] = [
            m.m[0][0], m.m[1][0], m.m[2][0], m.m[3][0], // [1, 0, 0, tx]
            m.m[0][1], m.m[1][1], m.m[2][1], m.m[3][1], // [0, 1, 0, ty]
            m.m[0][2], m.m[1][2], m.m[2][2], m.m[3][2], // [0, 0, 1, tz]
        ];
        // Translation should be at indices 3, 7, 11
        assert!((affine[3] - 1.0).abs() < f32::EPSILON, "tx");
        assert!((affine[7] - 2.0).abs() < f32::EPSILON, "ty");
        assert!((affine[11] - 3.0).abs() < f32::EPSILON, "tz");
        // Diagonal should be identity
        assert!((affine[0] - 1.0).abs() < f32::EPSILON, "r00");
        assert!((affine[5] - 1.0).abs() < f32::EPSILON, "r11");
        assert!((affine[10] - 1.0).abs() < f32::EPSILON, "r22");
    }
}
