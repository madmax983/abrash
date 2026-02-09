// GPU Triangle Binning Compute Shader
// Dispatches one thread per triangle to bin it into 32×32 pixel tiles

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

    // Depth bounds (8 bytes)
    float min_depth;         // offset 100
    float max_depth;         // offset 104
};

// Tile bin structure (1028 bytes)
// Each bin holds up to 256 triangle indices
struct TileBin
{
    uint count;              // Number of triangles in bin
    uint triangle_indices[510]; // Triangle indices (max 2048 bytes per D3D12 structured buffer)
};

// Constant buffer
cbuffer Constants : register(b0)
{
    uint g_TriangleCount;    // Total number of triangles
    uint g_TilesX;           // Number of tiles horizontally
    uint g_TilesY;           // Number of tiles vertically
    uint g_TileSize;         // Tile size in pixels (32)
};

// Input: Triangle data
StructuredBuffer<PreparedTriangle> g_Triangles : register(t0);

// Output: Tile bins
RWStructuredBuffer<TileBin> g_TileBins : register(u0);

// Compute shader entry point
// One thread per triangle, 64 threads per group
[numthreads(64, 1, 1)]
void BinTriangles(uint3 gid : SV_DispatchThreadID)
{
    // Check if this thread has a valid triangle
    if (gid.x >= g_TriangleCount)
    {
        return;
    }

    // Load triangle data
    PreparedTriangle tri = g_Triangles[gid.x];

    // Extract AABB bounds
    int aabb_min_x = tri.aabb_min_x;
    int aabb_min_y = tri.aabb_min_y;
    int aabb_max_x = tri.aabb_max_x;
    int aabb_max_y = tri.aabb_max_y;

    // Compute tile range that overlaps the triangle AABB
    // Clamp to [0, max] to handle edge cases
    int tx_min = max(0, aabb_min_x / (int)g_TileSize);
    int ty_min = max(0, aabb_min_y / (int)g_TileSize);
    int tx_max = min((int)g_TilesX - 1, aabb_max_x / (int)g_TileSize);
    int ty_max = min((int)g_TilesY - 1, aabb_max_y / (int)g_TileSize);

    // Iterate over all tiles that overlap the triangle
    for (int ty = ty_min; ty <= ty_max; ++ty)
    {
        for (int tx = tx_min; tx <= tx_max; ++tx)
        {
            // Compute linear tile index
            uint tile_index = ty * g_TilesX + tx;

            // Atomically increment the bin count and get the insertion offset
            uint insert_offset;
            InterlockedAdd(g_TileBins[tile_index].count, 1, insert_offset);

            // Check for bin overflow (max 510 triangles per bin)
            if (insert_offset < 510)
            {
                // Append triangle index to the bin
                g_TileBins[tile_index].triangle_indices[insert_offset] = gid.x;
            }
            // Note: If overflow occurs, the triangle is silently dropped
            // The CPU binning path should be used for scenes with very high triangle density
        }
    }
}
