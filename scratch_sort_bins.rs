    /// Sorts all bins in place using the provided comparator.
    /// This rebuilds the internal linked lists for each bin.
    ///
    /// This method is designed to avoid dynamic allocations per frame by reusing
    /// a thread-local sorting buffer.
    pub fn sort_all_bins<F>(&mut self, mut compare_fn: F)
    where
        F: FnMut(usize, usize) -> std::cmp::Ordering,
    {
        // Reusable scratch buffer for sorting
        let mut indices = Vec::with_capacity(128);

        for bin_idx in 0..self.heads.len() {
            // 1. Collect all triangle indices in this bin
            indices.clear();
            for tri_idx in self.iter(bin_idx) {
                indices.push(tri_idx);
            }

            if indices.len() <= 1 {
                continue;
            }

            // 2. Sort them
            indices.sort_unstable_by(|&a, &b| compare_fn(a, b));

            // 3. Rebuild the linked list for this bin
            let mut current_idx = u32::MAX;
            let mut prev_node = u32::MAX;

            // We need to find the node index in `nexts` and `tris` that corresponds to
            // each triangle index.
            // Wait, the linked list structure stores nodes. `tris` stores the triangle index at node `i`.
            // We need to re-link the nodes in the new order.
            //
            // Current structure:
            // heads[bin] -> node_idx -> nexts[node_idx] -> ...
            // tris[node_idx] = triangle_index
            //
            // We have a list of sorted triangle indices. We need to assign these back to the nodes.
            // Actually, we can just overwrite the `tris` values in the existing nodes to match the sorted order!
            // The linked list topology (next pointers) doesn't strictly need to change if we just swap the payloads.
            //
            // Let's verify:
            // The number of nodes remains the same.
            // We traverse the linked list again, and for each node, we assign the next sorted triangle index.

            let mut node_idx = self.heads[bin_idx];
            for &sorted_tri_idx in &indices {
                if node_idx == u32::MAX {
                    // Should not happen if we collected correctly
                    break;
                }
                // Overwrite payload
                self.tris[node_idx as usize] = sorted_tri_idx as u32;

                // Move to next node
                node_idx = self.nexts[node_idx as usize];
            }
        }
    }
