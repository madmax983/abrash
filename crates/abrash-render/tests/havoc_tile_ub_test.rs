use abrash_render::rasterizer::tile::{
    CompactScreenPoint, PreparedTriangle, PreparedTrianglesList,
};
//use rayon::prelude::*

#[test]
fn test_tile_uninit_read_ub() {
    let mut list = PreparedTrianglesList::new();
    list.push(PreparedTriangle {
        p0: CompactScreenPoint { x: 0, y: 0, z: 0.0 },
        p1: CompactScreenPoint { x: 1, y: 1, z: 0.0 },
        p2: CompactScreenPoint { x: 2, y: 2, z: 0.0 },
        dz_dx: 0.0,
        long_edge_is_left: false,
        color: 0,
        aabb_min_x: 0,
        aabb_min_y: 0,
        aabb_max_x: 2,
        aabb_max_y: 2,
        min_depth: 0.0,
        max_depth: 1.0,
    });
    // This will trigger UB when calling into_iter
    let _items: Vec<_> = list.into_iter().collect();
}
