//! Per-vertex bone influence data and skinned mesh.

use abrash_core::mesh::Mesh;

/// Per-vertex bone influence data (matches glTF `JOINTS_0` + `WEIGHTS_0`).
///
/// Each vertex has up to 4 bone influences. The joint indices refer to
/// joints in the associated [`crate::skeleton::Skeleton`], and the
/// corresponding weights should sum to approximately 1.0.
///
/// # Examples
/// ```
/// use abrash_skeletal::skin::SkinData;
///
/// let skin = SkinData {
///     joint_indices: vec![[0, 1, 0, 0]],
///     weights: vec![[0.8, 0.2, 0.0, 0.0]],
/// };
/// skin.validate(1, 2).unwrap(); // Validates 1 vertex against a 2-joint skeleton
/// ```
#[derive(Debug, Clone)]
pub struct SkinData {
    /// 4 joint indices per vertex.
    pub joint_indices: Vec<[u16; 4]>,
    /// 4 weights per vertex (should sum to ~1.0).
    pub weights: Vec<[f32; 4]>,
}

impl SkinData {
    /// Validate that this skin data is consistent with the given vertex and joint counts.
    ///
    /// Checks:
    /// - `joint_indices` length matches `vertex_count`
    /// - `weights` length matches `vertex_count`
    /// - All joint indices are < `joint_count`
    /// - All weight tuples sum to approximately 1.0 (tolerance: 0.01)
    ///
    /// # Errors
    ///
    /// Returns a descriptive error string on validation failure.
    pub fn validate(&self, vertex_count: usize, joint_count: usize) -> Result<(), String> {
        if self.joint_indices.len() != vertex_count {
            return Err(format!(
                "joint_indices length ({}) != vertex_count ({vertex_count})",
                self.joint_indices.len()
            ));
        }
        if self.weights.len() != vertex_count {
            return Err(format!(
                "weights length ({}) != vertex_count ({vertex_count})",
                self.weights.len()
            ));
        }

        for (v, indices) in self.joint_indices.iter().enumerate() {
            for (i, &idx) in indices.iter().enumerate() {
                // Only check joints with non-zero weight
                if self.weights[v][i] > 0.0 && idx as usize >= joint_count {
                    return Err(format!(
                        "vertex {v} joint_indices[{i}] = {idx} is out of range (joint_count = {joint_count})"
                    ));
                }
            }
        }

        for (v, w) in self.weights.iter().enumerate() {
            let sum: f32 = w.iter().sum();
            if (sum - 1.0).abs() > 0.01 {
                return Err(format!("vertex {v} weights sum to {sum}, expected ~1.0"));
            }
        }

        Ok(())
    }
}

/// A mesh augmented with skinning data.
#[derive(Debug, Clone)]
pub struct SkinnedMesh {
    /// The base mesh geometry.
    pub mesh: Mesh,
    /// Per-vertex bone influences.
    pub skin: SkinData,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_skin_data(vertex_count: usize) -> SkinData {
        SkinData {
            joint_indices: vec![[0, 0, 0, 0]; vertex_count],
            weights: vec![[1.0, 0.0, 0.0, 0.0]; vertex_count],
        }
    }

    #[test]
    fn valid_skin_passes_validation() {
        let skin = valid_skin_data(4);
        assert!(skin.validate(4, 2).is_ok());
    }

    #[test]
    fn mismatched_joint_indices_length_fails() {
        let skin = SkinData {
            joint_indices: vec![[0, 0, 0, 0]; 3],
            weights: vec![[1.0, 0.0, 0.0, 0.0]; 4],
        };
        let result = skin.validate(4, 2);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("joint_indices length"));
    }

    #[test]
    fn mismatched_weights_length_fails() {
        let skin = SkinData {
            joint_indices: vec![[0, 0, 0, 0]; 4],
            weights: vec![[1.0, 0.0, 0.0, 0.0]; 3],
        };
        let result = skin.validate(4, 2);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("weights length"));
    }

    #[test]
    fn out_of_range_joint_index_fails() {
        let skin = SkinData {
            joint_indices: vec![[5, 0, 0, 0]; 2],
            weights: vec![[1.0, 0.0, 0.0, 0.0]; 2],
        };
        let result = skin.validate(2, 3);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of range"));
    }

    #[test]
    fn zero_weight_joint_index_out_of_range_is_ok() {
        // Joint index 99 but weight is 0.0 — should be ignored.
        let skin = SkinData {
            joint_indices: vec![[0, 99, 0, 0]; 2],
            weights: vec![[1.0, 0.0, 0.0, 0.0]; 2],
        };
        assert!(skin.validate(2, 3).is_ok());
    }

    #[test]
    fn weights_not_summing_to_one_fails() {
        let skin = SkinData {
            joint_indices: vec![[0, 0, 0, 0]; 2],
            weights: vec![[0.5, 0.0, 0.0, 0.0]; 2],
        };
        let result = skin.validate(2, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("weights sum"));
    }

    #[test]
    fn weights_within_tolerance_pass() {
        let skin = SkinData {
            joint_indices: vec![[0, 1, 0, 0]; 1],
            weights: vec![[0.505, 0.505, 0.0, 0.0]; 1],
        };
        // Sum = 1.01, within 0.01 tolerance
        assert!(skin.validate(1, 2).is_ok());
    }

    #[test]
    fn multi_bone_weights_valid() {
        let skin = SkinData {
            joint_indices: vec![[0, 1, 2, 3]; 2],
            weights: vec![[0.25, 0.25, 0.25, 0.25]; 2],
        };
        assert!(skin.validate(2, 4).is_ok());
    }
}
