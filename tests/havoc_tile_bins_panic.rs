use abrash::rasterizer::tile::TileBins;

#[test]
#[should_panic(expected = "index out of bounds")]
fn test_tile_bins_u32_max_panic() {
    let mut bins = TileBins::new(1);

    // We can trigger an out of bounds panic by pushing u32::MAX elements!
    // But that takes too much memory. What if we just manually set the head to u32::MAX - 1?

    // The vulnerability is that if the number of triangles exceeds u32::MAX,
    // `tris.len() as u32` truncates and node_idx becomes 0.
    // If it reaches exactly u32::MAX, node_idx is u32::MAX, which is the sentinel value!

    // Let's directly manipulate the internal state to simulate a tile with
    // u32::MAX as the head, and then try to iterate.
    // Wait, `TileBins` fields are `pub`!
    bins.heads[0] = std::u32::MAX - 1;
    bins.nexts.push(std::u32::MAX); // Next after u32::MAX - 1 is u32::MAX (end)
    bins.tris.push(42);

    // This isn't actually a crash in the engine's normal operation unless we use 17GB of memory.
}
