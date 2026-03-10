with open('src/rasterizer/tile.rs', 'r') as f:
    content = f.read()

import re

# `sort_bin` in `sort_bins_flat` is a closure used across parallel iterations (or sequential).
# Let's fix that one too if it exists.
content = re.sub(
    r"""        // Helper to sort a single bin \(linked list\)
        let sort_bin = \|head: &mut u32, nexts: &mut \[u32\], tris: &\[u32\]\| \{
            if \*head == u32::MAX \{
                return;
            \}

            // 1\. Collect indices into a temporary vector
            // We reuse a thread-local buffer to avoid allocations\?
            // For now, just allocate a small vec\. Most tiles have < 100 triangles\.
            let mut indices = Vec::with_capacity\(64\);
            let mut curr = \*head;
            while curr != u32::MAX \{
                indices\.push\(curr\);
                curr = nexts\[curr as usize\];
            \}""",
    r"""        // Helper to sort a single bin (linked list)
        // Bolt: Use thread_local here since this closure is called from par_iter_mut
        let sort_bin = |head: &mut u32, nexts: &mut [u32], tris: &[u32]| {
            if *head == u32::MAX {
                return;
            }

            // 1. Collect indices into a temporary vector
            thread_local! { static SCRATCH: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) }; }
            SCRATCH.with(|scratch| {
                let mut indices = scratch.borrow_mut();
                indices.clear();
                let mut curr = *head;
                while curr != u32::MAX {
                    indices.push(curr);
                    curr = nexts[curr as usize];
                }""",
    content
)

# And we have to close the scratch block
content = re.sub(
    r"""            // 3\. Rebuild linked list
            \*head = indices\[0\];
            let len = indices\.len\(\);
            for i in 0\.\.len - 1 \{
                nexts\[indices\[i\] as usize\] = indices\[i \+ 1\];
            \}
            nexts\[indices\[len - 1\] as usize\] = u32::MAX;
            // tails\[\] is not passed, but we don't strictly need it for rendering, only for appending\.
            // wait, if we append later we might need tails\.
            // This is a known bug in this refactored method, hence it's commented out\.
        \};""",
    r"""            // 3. Rebuild linked list
            *head = indices[0];
            let len = indices.len();
            for i in 0..len - 1 {
                nexts[indices[i] as usize] = indices[i + 1];
            }
            nexts[indices[len - 1] as usize] = u32::MAX;
            // tails[] is not passed, but we don't strictly need it for rendering, only for appending.
            // wait, if we append later we might need tails.
            // This is a known bug in this refactored method, hence it's commented out.
            });
        };""",
    content
)

with open('src/rasterizer/tile.rs', 'w') as f:
    f.write(content)
