import sys

filepath = "crates/abrash-render/src/render_api/cpu_renderer.rs"
with open(filepath, "r") as f:
    content = f.read()

old_code = """
            // Use par_extend to directly populate batches without an intermediate Vec allocation.
            draw_list
                .batches
                .par_extend(frame.commands.par_iter().zip(ranges.as_slice()).map(
                    |(cmd, &(start, end))| {
                        let cpu_mesh = self.meshes.get(from_mesh_handle(cmd.mesh)).unwrap();
                        let material = self
                            .materials
                            .get(from_material_handle(cmd.material))
                            .unwrap();

                        let mvp = cmd.transform * view_proj;
                        let mesh = &cpu_mesh.mesh;

                        // SAFETY: `ranges` ensures disjoint segments of the allocated buffer.
                        // The buffer is pre-allocated with `total_vertices` capacity.
                        unsafe {
                            let offset_ptr = (ptr as *mut (crate::math::Vec3, f32)).add(start);
                            // We cast `offset_ptr` to `*mut std::mem::MaybeUninit` to pass into `transform_points_uninit`.
                            let slice = std::slice::from_raw_parts_mut(
                                offset_ptr
                                    .cast::<std::mem::MaybeUninit<(crate::math::Vec3, f32)>>(),
                                mesh.vertices.len(),
                            );
                            mvp.transform_points_uninit(&mesh.vertices, slice);
                        }

                        DrawBatch::new(
                            start..end,
                            std::sync::Arc::clone(&cpu_mesh.shared_indices),
                            material.color,
                        )
                    },
                ));
"""

new_code = """
            // Use collect to evaluate the result, failing fast if a stale handle is found.
            let batches: Result<Vec<_>, _> = frame.commands.par_iter().zip(ranges.as_slice()).map(
                |(cmd, &(start, end))| {
                    let cpu_mesh = self
                        .meshes
                        .get(from_mesh_handle(cmd.mesh))
                        .ok_or(RenderError::StaleHandle("mesh"))?;
                    let material = self
                        .materials
                        .get(from_material_handle(cmd.material))
                        .ok_or(RenderError::StaleHandle("material"))?;

                    let mvp = cmd.transform * view_proj;
                    let mesh = &cpu_mesh.mesh;

                    // SAFETY: `ranges` ensures disjoint segments of the allocated buffer.
                    // The buffer is pre-allocated with `total_vertices` capacity.
                    unsafe {
                        let offset_ptr = (ptr as *mut (crate::math::Vec3, f32)).add(start);
                        // We cast `offset_ptr` to `*mut std::mem::MaybeUninit` to pass into `transform_points_uninit`.
                        let slice = std::slice::from_raw_parts_mut(
                            offset_ptr
                                .cast::<std::mem::MaybeUninit<(crate::math::Vec3, f32)>>(),
                            mesh.vertices.len(),
                        );
                        mvp.transform_points_uninit(&mesh.vertices, slice);
                    }

                    Ok(DrawBatch::new(
                        start..end,
                        std::sync::Arc::clone(&cpu_mesh.shared_indices),
                        material.color,
                    ))
                },
            ).collect();

            let batches = batches?;
            draw_list.batches.extend(batches);
"""

if old_code in content:
    content = content.replace(old_code, new_code)
    with open(filepath, "w") as f:
        f.write(content)
    print("Patched successfully")
else:
    print("Old code not found")
