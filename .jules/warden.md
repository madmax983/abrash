**2023-10-27 - [SoftBody SDF Collision Out-Of-Bounds Panic]
**Threat:** [Out-of-bounds indexing in `SoftBody::collide_sdf` causing application panic (DoS) when the mesh vertices and velocities vectors are desynchronized by the user.]
**Defense:** [Added an explicit structural synchronization check at the beginning of `collide_sdf` that verifies `self.mesh.vertices.len() == self.velocities.len()`, logging an error and returning early if a mismatch is detected.]
