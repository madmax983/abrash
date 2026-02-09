// GPU Coarse Triangle Binning Compute Shader (128×128 bins)
// First pass of two-level hierarchical binning
// Bins triangles to 128×128 pixel coarse bins for Hi-Z culling

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

// Coarse bin covering 128×128 pixels
struct CoarseBin
{
    uint triangle_count;     // Atomic counter
    uint triangle_indices[510]; // Triangle indices (same capacity as fine tiles)
    // Note: Depth range (min/max) computed on CPU after readback to avoid atomic float issues
};

// Shader resources
StructuredBuffer<PreparedTriangle> g_Triangles : register(t0);
RWStructuredBuffer<CoarseBin> g_CoarseBins : register(u0);

cbuffer Constants : register(b0)
{
    uint g_TriangleCount;
    uint g_CoarseBinsX;      // Number of coarse bins horizontally
    uint g_CoarseBinsY;      // Number of coarse bins vertically
    uint g_CoarseBinSize;    // 128 pixels
};

// Thread per triangle
[numthreads(64, 1, 1)]
void BinCoarse(uint3 gid : SV_DispatchThreadID)
{
    uint tri_idx = gid.x;

    // Bounds check
    if (tri_idx >= g_TriangleCount)
    {
        return;
    }

    // Load triangle
    PreparedTriangle tri = g_Triangles[tri_idx];
    int aabb_min_x = tri.aabb_min_x;
    int aabb_min_y = tri.aabb_min_y;
    int aabb_max_x = tri.aabb_max_x;
    int aabb_max_y = tri.aabb_max_y;

    // Compute which coarse bins this triangle overlaps
    // Clamp to valid bin range
    uint cx_min = (uint)max(0, aabb_min_x / (int)g_CoarseBinSize);
    uint cy_min = (uint)max(0, aabb_min_y / (int)g_CoarseBinSize);
    uint cx_max = min((uint)(aabb_max_x / (int)g_CoarseBinSize), g_CoarseBinsX - 1);
    uint cy_max = min((uint)(aabb_max_y / (int)g_CoarseBinSize), g_CoarseBinsY - 1);

    // Bin triangle to all overlapping coarse bins
    for (uint cy = cy_min; cy <= cy_max; ++cy)
    {
        for (uint cx = cx_min; cx <= cx_max; ++cx)
        {
            uint bin_index = cy * g_CoarseBinsX + cx;

            // Atomic append triangle index to bin
            uint insert_offset;
            InterlockedAdd(g_CoarseBins[bin_index].triangle_count, 1, insert_offset);

            // Check for bin overflow (max 510 triangles per bin)
            if (insert_offset < 510)
            {
                // Append triangle index to the bin
                g_CoarseBins[bin_index].triangle_indices[insert_offset] = tri_idx;
                // Depth range will be computed on CPU after readback
            }
            // Note: If overflow occurs, the triangle is silently dropped
            // This should be rare with 510 triangle limit and 128×128 bins
        }
    }
}
