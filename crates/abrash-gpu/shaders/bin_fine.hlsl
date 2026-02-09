// GPU Fine Triangle Binning Compute Shader (32×32 tiles)
// Second pass of two-level hierarchical binning
// Refines visible coarse bins to 32×32 pixel fine tiles for rasterization

// PreparedTriangle structure (80 bytes total)
// Must match Rust layout exactly for StructuredBuffer access
struct PreparedTriangle
{
    // Floating-point vertices (24 bytes)
    float3 p0;               // offset 0
    float3 p1;               // offset 12
    float3 p2;               // offset 24

    // Fixed-point vertices 24.8 format (36 bytes)
    int3 p0_fixed;           // offset 36
    int3 p1_fixed;           // offset 48
    int3 p2_fixed;           // offset 60

    // Edge walking parameters (12 bytes)
    float dz_dx;             // offset 72
    uint long_edge_is_left;  // offset 76
    uint color;              // offset 80

    // Screen-space AABB (16 bytes)
    int aabb_min_x;          // offset 84
    int aabb_min_y;          // offset 88
    int aabb_max_x;          // offset 92
    int aabb_max_y;          // offset 96

    // Depth range (8 bytes)
    float min_depth;         // offset 100
    float max_depth;         // offset 104
};

// Visible coarse bin (output from Hi-Z culling)
struct VisibleCoarseBin
{
    uint coarse_bin_index;   // Original bin index (for computing bounds)
    uint triangle_count;
    uint triangle_indices[510];
};

// Fine tile covering 32×32 pixels
struct FineTile
{
    uint triangle_count;     // Atomic counter
    uint triangle_indices[510]; // Triangle indices
};

// Shader resources
StructuredBuffer<PreparedTriangle> g_Triangles : register(t0);
StructuredBuffer<VisibleCoarseBin> g_VisibleBins : register(t1);
RWStructuredBuffer<FineTile> g_FineTiles : register(u0);

cbuffer Constants : register(b0)
{
    uint g_VisibleBinCount;  // Number of visible coarse bins
    uint g_CoarseBinsX;      // Number of coarse bins horizontally
    uint g_CoarseBinsY;      // Number of coarse bins vertically
    uint g_CoarseBinSize;    // 128 pixels
    uint g_TilesX;           // Number of fine tiles horizontally
    uint g_TilesY;           // Number of fine tiles vertically
    uint g_TileSize;         // 32 pixels
};

// Thread per visible coarse bin
[numthreads(64, 1, 1)]
void BinFine(uint3 gid : SV_DispatchThreadID)
{
    uint visible_bin_idx = gid.x;

    // Bounds check
    if (visible_bin_idx >= g_VisibleBinCount)
    {
        return;
    }

    // Load visible coarse bin
    VisibleCoarseBin coarse_bin = g_VisibleBins[visible_bin_idx];
    uint coarse_bin_index = coarse_bin.coarse_bin_index;
    uint triangle_count = coarse_bin.triangle_count;

    // Compute coarse bin screen bounds from bin index
    uint coarse_bin_x = coarse_bin_index % g_CoarseBinsX;
    uint coarse_bin_y = coarse_bin_index / g_CoarseBinsX;

    // Screen-space bounds of this coarse bin (in pixels)
    int coarse_min_x = (int)(coarse_bin_x * g_CoarseBinSize);
    int coarse_min_y = (int)(coarse_bin_y * g_CoarseBinSize);
    int coarse_max_x = (int)((coarse_bin_x + 1) * g_CoarseBinSize - 1);
    int coarse_max_y = (int)((coarse_bin_y + 1) * g_CoarseBinSize - 1);

    // Compute fine tile range covered by this coarse bin
    uint tile_x_min = coarse_bin_x * (g_CoarseBinSize / g_TileSize);
    uint tile_y_min = coarse_bin_y * (g_CoarseBinSize / g_TileSize);
    uint tile_x_max = tile_x_min + (g_CoarseBinSize / g_TileSize) - 1;
    uint tile_y_max = tile_y_min + (g_CoarseBinSize / g_TileSize) - 1;

    // Clamp to valid tile range
    tile_x_max = min(tile_x_max, g_TilesX - 1);
    tile_y_max = min(tile_y_max, g_TilesY - 1);

    // Process each triangle in this coarse bin
    for (uint i = 0; i < triangle_count; ++i)
    {
        // Load triangle index and triangle data
        uint tri_idx = coarse_bin.triangle_indices[i];
        PreparedTriangle tri = g_Triangles[tri_idx];

        // Triangle AABB in screen space
        int tri_min_x = tri.aabb_min_x;
        int tri_min_y = tri.aabb_min_y;
        int tri_max_x = tri.aabb_max_x;
        int tri_max_y = tri.aabb_max_y;

        // Intersect triangle AABB with coarse bin bounds
        // (Triangles are guaranteed to overlap coarse bin, but may not fill it)
        tri_min_x = max(tri_min_x, coarse_min_x);
        tri_min_y = max(tri_min_y, coarse_min_y);
        tri_max_x = min(tri_max_x, coarse_max_x);
        tri_max_y = min(tri_max_y, coarse_max_y);

        // Compute which fine tiles this triangle overlaps within coarse bin
        uint tx_min = (uint)max(0, tri_min_x / (int)g_TileSize);
        uint ty_min = (uint)max(0, tri_min_y / (int)g_TileSize);
        uint tx_max = (uint)(tri_max_x / (int)g_TileSize);
        uint ty_max = (uint)(tri_max_y / (int)g_TileSize);

        // Clamp to coarse bin's fine tile range
        tx_min = max(tx_min, tile_x_min);
        ty_min = max(ty_min, tile_y_min);
        tx_max = min(tx_max, tile_x_max);
        ty_max = min(ty_max, tile_y_max);

        // Bin triangle to all overlapping fine tiles
        for (uint ty = ty_min; ty <= ty_max; ++ty)
        {
            for (uint tx = tx_min; tx <= tx_max; ++tx)
            {
                uint tile_index = ty * g_TilesX + tx;

                // Atomic append triangle index to fine tile
                uint insert_offset;
                InterlockedAdd(g_FineTiles[tile_index].triangle_count, 1, insert_offset);

                // Check for tile overflow (max 510 triangles per tile)
                if (insert_offset < 510)
                {
                    // Append triangle index to the fine tile
                    g_FineTiles[tile_index].triangle_indices[insert_offset] = tri_idx;
                }
                // Note: If overflow occurs, the triangle is silently dropped
                // This should be rare with 510 triangle limit and 32×32 tiles
            }
        }
    }
}
