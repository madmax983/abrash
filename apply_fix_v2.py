import sys

# Read the file
with open('src/rasterizer.rs', 'r') as f:
    lines = f.readlines()

# Helper to match and replace lines
def replace_block(lines, start_marker, end_marker, new_lines):
    start_idx = -1
    for i, line in enumerate(lines):
        if start_marker in line:
            start_idx = i
            break

    if start_idx == -1:
        print(f"Could not find start marker: {start_marker.strip()}")
        return lines

    # Find end of block (basic brace counting or just context)
    # For get_pixel_bilinear_texel, we want to replace the whole function body + signature
    # It starts at  and ends at the closing brace of the function.

    # Actually, for reliability, let's look for specific unique lines.

    return lines

content = "".join(lines)

# 1. Texture::get_pixel_bilinear_texel
# We know the specific signature line
sig = "    pub fn get_pixel_bilinear_texel(&self, u_tex: f32, v_tex: f32) -> u32 {"
if sig not in content:
    print("Signature not found!")
    # Maybe it's already patched?
    if "get_pixel_bilinear_fixed" in content:
        print("Already patched Texture?")
else:
    # Find the end of the function. It's indentation based.
    start = content.find(sig)
    # Scan for matching brace? Or just replace the known body structure?
    # The body is quite long.
    # Let's try to locate the start and the end based on the next function definition or EOF.
    # Next function is get_pixel_texel
    next_sig = "    pub fn get_pixel_texel(&self, x: i32, y: i32) -> u32 {"
    end = content.find(next_sig)

    if start != -1 and end != -1:
        # We need to backtrack from  to include the closing brace of previous function and some whitespace.
        # Actually, let's just replace the substring.

        # The block to replace is from  up to  (exclusive of next_sig, inclusive of previous closing brace).

        # We need to construct the new block.
        new_block = """    /// Sample texture using bilinear interpolation with texel coordinates
    #[inline]
    pub fn get_pixel_bilinear_texel(&self, u_tex: f32, v_tex: f32) -> u32 {
        // Convert to 24.8 fixed point
        // 0.5 in 24.8 is 128
        let u_fixed = (u_tex * 256.0) as i32;
        let v_fixed = (v_tex * 256.0) as i32;
        self.get_pixel_bilinear_fixed(u_fixed, v_fixed)
    }

    /// Sample texture using bilinear interpolation with 24.8 fixed point texel coordinates
    #[inline]
    pub fn get_pixel_bilinear_fixed(&self, u_fixed: i32, v_fixed: i32) -> u32 {
        let u_img_fixed = u_fixed - 128;
        let v_img_fixed = v_fixed - 128;

        // Weights (0..256)
        let wx = (u_img_fixed & 0xFF) as u32;
        let wy = (v_img_fixed & 0xFF) as u32;
        let inv_wx = 256 - wx;
        let inv_wy = 256 - wy;

        // Coordinates
        let w_i32 = self.width as i32 - 1;
        let h_i32 = self.height as i32 - 1;

        // Arithmetic shift preserves sign (floor behavior for negative numbers)
        let x0_raw = u_img_fixed >> 8;
        let y0_raw = v_img_fixed >> 8;

        let x0 = x0_raw.clamp(0, w_i32) as usize;
        let y0 = y0_raw.clamp(0, h_i32) as usize;
        let x1 = (x0_raw + 1).clamp(0, w_i32) as usize;
        let y1 = (y0_raw + 1).clamp(0, h_i32) as usize;

        let width_usize = self.width as usize;
        let row0 = y0 * width_usize;
        let row1 = y1 * width_usize;

        // SAFETY: We clamped coordinates to valid ranges [0, width-1] / [0, height-1]
        let (c00, c10, c01, c11) = unsafe {
            (
                *self.pixels.get_unchecked(row0 + x0),
                *self.pixels.get_unchecked(row0 + x1),
                *self.pixels.get_unchecked(row1 + x0),
                *self.pixels.get_unchecked(row1 + x1),
            )
        };

        // Function to blend two colors with weight w using SWAR (SIMD Within A Register)
        // Blends R/B and A/G in parallel
        let blend = |c0: u32, c1: u32, w: u32, inv_w: u32| -> u32 {
            let rb0 = c0 & 0x00FF00FF;
            let ag0 = (c0 >> 8) & 0x00FF00FF;
            let rb1 = c1 & 0x00FF00FF;
            let ag1 = (c1 >> 8) & 0x00FF00FF;

            let rb = ((rb0 * inv_w + rb1 * w) >> 8) & 0x00FF00FF;
            let ag = ((ag0 * inv_w + ag1 * w) >> 8) & 0x00FF00FF;

            rb | (ag << 8)
        };

        let top = blend(c00, c10, wx, inv_wx);
        let bottom = blend(c01, c11, wx, inv_wx);
        let final_color = blend(top, bottom, wy, inv_wy);

        // Ensure alpha is 0xFF
        final_color | 0xFF000000
    }

    /// Sample texture using texel coordinates
"""
        # We need to capture the exact text to replace.
        # It starts at
        # And ends just before

        start_marker = "    /// Sample texture using bilinear interpolation with texel coordinates"
        end_marker = "    /// Sample texture using texel coordinates"

        s_idx = content.find(start_marker)
        e_idx = content.find(end_marker)

        if s_idx != -1 and e_idx != -1:
            content = content[:s_idx] + new_block + content[e_idx + len("    /// Sample texture using texel coordinates"):]
            # Wait,  ends with the header of the next function?
            # Yes, .
            # So we should strip that from new_block if we are keeping the existing one, OR replace up to it.

            # Let's adjust new_block to NOT include the next function header.
            new_block = new_block.strip()
            # Remove the last line (the next function header)
            new_block = new_block[:new_block.rfind('\n')]
            new_block = new_block[:new_block.rfind('\n')]
            # Actually, string matching is tricky.

            # Use fixed strings.
            pass

# Let's try a simpler approach for the second replacement first, as it's more self contained.
bilinear_marker = "FilterMode::Bilinear => {"
if bilinear_marker in content:
    # Find the block
    start_idx = content.find(bilinear_marker)
    # The block ends with a closing brace indentation.
    # It looks like:
    #             FilterMode::Bilinear => {
    #                 ...
    #             }

    # We can try to replace the inner loop.
    inner_loop_start = "let mut u_tex = u_tex_start;"
    inner_loop_end = "v_tex += dv_tex_step;"

    start_inner = content.find(inner_loop_start, start_idx)
    end_inner = content.find(inner_loop_end, start_idx)

    if start_inner != -1 and end_inner != -1:
        # Include the closing brace of the loop?
        # The loop looks like:
        #                 for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        #                     if z < *depth_val {
        #                         *depth_val = z;
        #                         *pixel = texture.get_pixel_bilinear_texel(u_tex, v_tex);
        #                     }
        #                     z += gradients.dz_dx;
        #                     u_tex += du_tex_step;
        #                     v_tex += dv_tex_step;
        #                 }

        # We want to replace the whole block under FilterMode::Bilinear.
        pass

# I will just write the corrected file content completely.
# I know the structure of . I can read it, find the lines, and splice.

lines = open('src/rasterizer.rs', 'r').readlines()
new_lines = []
skip = False
skip_until = ""

i = 0
while i < len(lines):
    line = lines[i]

    # Refactor Texture::get_pixel_bilinear_texel
    if "pub fn get_pixel_bilinear_texel(&self, u_tex: f32, v_tex: f32) -> u32 {" in line:
        # We are at the function start.
        # We want to keep this signature, but change the body.
        # AND add the new function after it.

        # Actually, easier to replace the whole function block.
        # Scan forward to find the end of this function (before get_pixel_texel)
        j = i + 1
        while j < len(lines):
            if "pub fn get_pixel_texel(&self, x: i32, y: i32) -> u32 {" in lines[j]:
                break
            j += 1

        # lines[i:j] is the function (roughly, minus the comments of next function).
        # We replace lines[i:j-1] (assuming j-1 is the closing brace line or empty line)

        # Let's insert the new code.
        new_code = """    #[inline]
    pub fn get_pixel_bilinear_texel(&self, u_tex: f32, v_tex: f32) -> u32 {
        // Convert to 24.8 fixed point
        // 0.5 in 24.8 is 128
        let u_fixed = (u_tex * 256.0) as i32;
        let v_fixed = (v_tex * 256.0) as i32;
        self.get_pixel_bilinear_fixed(u_fixed, v_fixed)
    }

    /// Sample texture using bilinear interpolation with 24.8 fixed point texel coordinates
    #[inline]
    pub fn get_pixel_bilinear_fixed(&self, u_fixed: i32, v_fixed: i32) -> u32 {
        let u_img_fixed = u_fixed - 128;
        let v_img_fixed = v_fixed - 128;

        // Weights (0..256)
        let wx = (u_img_fixed & 0xFF) as u32;
        let wy = (v_img_fixed & 0xFF) as u32;
        let inv_wx = 256 - wx;
        let inv_wy = 256 - wy;

        // Coordinates
        let w_i32 = self.width as i32 - 1;
        let h_i32 = self.height as i32 - 1;

        // Arithmetic shift preserves sign (floor behavior for negative numbers)
        let x0_raw = u_img_fixed >> 8;
        let y0_raw = v_img_fixed >> 8;

        let x0 = x0_raw.clamp(0, w_i32) as usize;
        let y0 = y0_raw.clamp(0, h_i32) as usize;
        let x1 = (x0_raw + 1).clamp(0, w_i32) as usize;
        let y1 = (y0_raw + 1).clamp(0, h_i32) as usize;

        let width_usize = self.width as usize;
        let row0 = y0 * width_usize;
        let row1 = y1 * width_usize;

        // SAFETY: We clamped coordinates to valid ranges [0, width-1] / [0, height-1]
        let (c00, c10, c01, c11) = unsafe {
            (
                *self.pixels.get_unchecked(row0 + x0),
                *self.pixels.get_unchecked(row0 + x1),
                *self.pixels.get_unchecked(row1 + x0),
                *self.pixels.get_unchecked(row1 + x1),
            )
        };

        // Function to blend two colors with weight w using SWAR (SIMD Within A Register)
        // Blends R/B and A/G in parallel
        let blend = |c0: u32, c1: u32, w: u32, inv_w: u32| -> u32 {
            let rb0 = c0 & 0x00FF00FF;
            let ag0 = (c0 >> 8) & 0x00FF00FF;
            let rb1 = c1 & 0x00FF00FF;
            let ag1 = (c1 >> 8) & 0x00FF00FF;

            let rb = ((rb0 * inv_w + rb1 * w) >> 8) & 0x00FF00FF;
            let ag = ((ag0 * inv_w + ag1 * w) >> 8) & 0x00FF00FF;

            rb | (ag << 8)
        };

        let top = blend(c00, c10, wx, inv_wx);
        let bottom = blend(c01, c11, wx, inv_wx);
        let final_color = blend(top, bottom, wy, inv_wy);

        // Ensure alpha is 0xFF
        final_color | 0xFF000000
    }

"""
        new_lines.append(new_code)

        # Skip original body lines.
        # Start matching at j, but backtrack to find the comments for next function.
        # "    /// Sample texture using texel coordinates" is at j-2?

        # Let's find the start of the next function's comment block.
        k = j - 1
        while k > i:
            if "///" in lines[k]:
                k -= 1
            else:
                break
        # k is now the line before the comments of next function.

        i = k + 1
        continue

    # Optimize loop
    if "FilterMode::Bilinear => {" in line:
        new_lines.append(line)
        # Skip the old body lines until the closing brace of this block.
        # Old body:
        #                 let mut u_tex = u_tex_start;
        #                 let mut v_tex = v_tex_start;
        #                 for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
        #                     if z < *depth_val {
        #                         *depth_val = z;
        #                         *pixel = texture.get_pixel_bilinear_texel(u_tex, v_tex);
        #                     }
        #                     z += gradients.dz_dx;
        #                     u_tex += du_tex_step;
        #                     v_tex += dv_tex_step;
        #                 }
        #             }

        # New body:
        new_body = """                // Fixed point optimization for Bilinear
                // Use 16.16 for accumulation to maintain precision, then downshift to 24.8 for sampling
                let mut u_fix = (u_tex_start * 65536.0) as i32;
                let mut v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
                    if z < *depth_val {
                        *depth_val = z;
                        // Convert 16.16 to 24.8 (x >> 8)
                        *pixel = texture.get_pixel_bilinear_fixed(u_fix >> 8, v_fix >> 8);
                    }
                    z += gradients.dz_dx;
                    u_fix = u_fix.wrapping_add(du_fix);
                    v_fix = v_fix.wrapping_add(dv_fix);
                }
            }
"""
        new_lines.append(new_body)

        # Now skip old lines
        i += 1
        while i < len(lines):
            # Check for closing brace of the block.
            # The closing brace is indented 12 spaces (3 tabs).
            if lines[i].strip() == "}" and lines[i].startswith("            }"):
                # This is the closing brace we just appended in new_body.
                # So we should skip it in the input.
                i += 1
                break
            i += 1
        continue

    new_lines.append(line)
    i += 1

with open('src/rasterizer.rs', 'w') as f:
    f.writelines(new_lines)

print("Applied V2 fix.")
