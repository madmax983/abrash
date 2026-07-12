import re

with open("crates/abrash-render/src/rasterizer/flat.rs", "r") as f:
    content = f.read()

content = content.replace("""    // Scalar tail
    let mut z = z_start + (i as f32) * dz_dx;
    for (depth_val, pixel) in zb_slice[i..len].iter_mut().zip(fb_slice[i..len].iter_mut()) {
        if z < *depth_val {
            *depth_val = z;
            *pixel = color;
        }
        z += dz_dx;
    }""", """    // Scalar tail
    let mut z = z_start + (i as f32) * dz_dx;
    for j in i..len {
        unsafe {
            let depth_val = zb_slice.get_unchecked_mut(j);
            if z < *depth_val {
                *depth_val = z;
                *fb_slice.get_unchecked_mut(j) = color;
            }
        }
        z += dz_dx;
    }""")

content = content.replace("""        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                *depth_val = z;
                *pixel = color;
            }
            z += dz_dx;
        }""", """        for j in 0..fb_slice.len() {
            unsafe {
                let depth_val = zb_slice.get_unchecked_mut(j);
                if z < *depth_val {
                    *depth_val = z;
                    *fb_slice.get_unchecked_mut(j) = color;
                }
            }
            z += dz_dx;
        }""")

content = content.replace("""    for (depth_val, pixel) in zb_slice[i..len].iter_mut().zip(fb_slice[i..len].iter_mut()) {
        if z < *depth_val {
            let dest = *pixel;

            let rb_dest = dest & 0x00FF_00FF;
            let ag_dest = (dest >> 8) & 0x00FF_00FF;

            let rb = ((rb_src_scaled_u32 + rb_dest * inv_alpha_u32) >> 8) & 0x00FF_00FF;
            let ag = ((ag_src_scaled_u32 + ag_dest * inv_alpha_u32) >> 8) & 0x00FF_00FF;

            *pixel = rb | (ag << 8);
        }
        z += dz_dx;
    }""", """    for j in i..len {
        unsafe {
            let depth_val = zb_slice.get_unchecked_mut(j);
            if z < *depth_val {
                let pixel = fb_slice.get_unchecked_mut(j);
                let dest = *pixel;

                let rb_dest = dest & 0x00FF_00FF;
                let ag_dest = (dest >> 8) & 0x00FF_00FF;

                let rb = ((rb_src_scaled_u32 + rb_dest * inv_alpha_u32) >> 8) & 0x00FF_00FF;
                let ag = ((ag_src_scaled_u32 + ag_dest * inv_alpha_u32) >> 8) & 0x00FF_00FF;

                *pixel = rb | (ag << 8);
            }
        }
        z += dz_dx;
    }""")

content = content.replace("""        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            // Test Z but do not write Z for transparent pixels
            if z < *depth_val {
                let dest = *pixel;

                let rb_dest = dest & 0x00FF_00FF;
                let ag_dest = (dest >> 8) & 0x00FF_00FF;

                // (src * alpha + dest * (255 - alpha)) >> 8
                let rb = ((rb_src_scaled + rb_dest * inv_alpha) >> 8) & 0x00FF_00FF;
                let ag = ((ag_src_scaled + ag_dest * inv_alpha) >> 8) & 0x00FF_00FF;

                *pixel = rb | (ag << 8);
            }
            z += dz_dx;
        }""", """        for j in 0..fb_slice.len() {
            unsafe {
                let depth_val = zb_slice.get_unchecked_mut(j);
                // Test Z but do not write Z for transparent pixels
                if z < *depth_val {
                    let pixel = fb_slice.get_unchecked_mut(j);
                    let dest = *pixel;

                    let rb_dest = dest & 0x00FF_00FF;
                    let ag_dest = (dest >> 8) & 0x00FF_00FF;

                    // (src * alpha + dest * (255 - alpha)) >> 8
                    let rb = ((rb_src_scaled + rb_dest * inv_alpha) >> 8) & 0x00FF_00FF;
                    let ag = ((ag_src_scaled + ag_dest * inv_alpha) >> 8) & 0x00FF_00FF;

                    *pixel = rb | (ag << 8);
                }
            }
            z += dz_dx;
        }""")

with open("crates/abrash-render/src/rasterizer/flat.rs", "w") as f:
    f.write(content)
