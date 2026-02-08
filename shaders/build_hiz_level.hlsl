// GPU Hi-Z Pyramid Level Build Compute Shader
// Builds one level of the Hi-Z pyramid using 2×2 min-reduction
// Each thread processes one output pixel by sampling 2×2 from the source level

// Shader resources
Texture2D<float> g_SourceLevel : register(t0);      // Source level (N-1)
RWTexture2D<float> g_DestLevel : register(u0);      // Destination level (N)

// Constant buffer
cbuffer Constants : register(b0)
{
    uint g_SourceWidth;   // Source level width
    uint g_SourceHeight;  // Source level height
    uint g_DestWidth;     // Destination level width
    uint g_DestHeight;    // Destination level height
};

// Compute shader entry point
// Workgroup size: 8×8 threads per group
// Each thread computes one output pixel
[numthreads(8, 8, 1)]
void BuildHiZLevel(uint3 gid : SV_DispatchThreadID)
{
    uint x = gid.x;
    uint y = gid.y;

    // Bounds check - discard threads outside destination dimensions
    if (x >= g_DestWidth || y >= g_DestHeight)
    {
        return;
    }

    // Source coordinates for 2×2 region
    // Each destination pixel corresponds to a 2×2 quad in the source
    uint src_x = x * 2;
    uint src_y = y * 2;

    // Load 2×2 quad from source level using exact pixel access (no filtering)
    // Texture2D.Load() uses integer coordinates: Load(int3(x, y, mipLevel))
    float d00 = g_SourceLevel.Load(int3(src_x,     src_y,     0));
    float d10 = g_SourceLevel.Load(int3(src_x + 1, src_y,     0));
    float d01 = g_SourceLevel.Load(int3(src_x,     src_y + 1, 0));
    float d11 = g_SourceLevel.Load(int3(src_x + 1, src_y + 1, 0));

    // Handle edge cases: odd dimensions
    // If the source dimension is odd, the last row/column won't have a neighbor
    // Clamp by duplicating the edge pixel (matches CPU implementation behavior)
    if (src_x + 1 >= g_SourceWidth)
    {
        // Right edge: no d10 or d11, duplicate d00 and d01
        d10 = d00;
        d11 = d01;
    }
    if (src_y + 1 >= g_SourceHeight)
    {
        // Bottom edge: no d01 or d11, duplicate d00 and d10
        d01 = d00;
        d11 = d10;
    }

    // Compute minimum depth across 2×2 quad
    // Conservative occlusion: store the CLOSEST depth from the 2×2 region
    float min_depth = min(min(d00, d10), min(d01, d11));

    // Write result to destination level
    g_DestLevel[uint2(x, y)] = min_depth;
}
