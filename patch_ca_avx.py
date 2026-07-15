import sys

with open("crates/abrash-render/src/post_process/filters.rs", "r") as f:
    content = f.read()

def get_avx():
    start = -1
    end = -1
    lines = content.split('\n')
    for i, line in enumerate(lines):
        if "pub unsafe fn apply_chromatic_aberration_avx2(" in line:
            start = i
        if start != -1 and "pub unsafe fn apply_sobel_avx2(" in line:
            end = i
            break

    if start != -1 and end != -1:
        return '\n'.join(lines[start:end])
    return ""

old_avx = get_avx()

if old_avx.endswith('    #[target_feature(enable = "avx2")]'):
    old_avx = old_avx[:-38].rstrip()

new_avx = """    pub unsafe fn apply_chromatic_aberration_avx2(
        pixels: &mut [u32],
        width: usize,
        _height: usize,
        offset: usize,
    ) {
        #[cfg(feature = "parallel")]
        let row_iter = pixels.par_chunks_exact_mut(width);
        #[cfg(not(feature = "parallel"))]
        let row_iter = pixels.chunks_exact_mut(width);

        row_iter.for_each(|row_pixels| {
            CA_BUFFER.with(|buf| {
                let mut row_buffer = buf.borrow_mut();
                if row_buffer.len() < width {
                    row_buffer.resize(width, 0);
                }

                let mask_r = unsafe { _mm256_set1_epi32(0x00FF_0000) };
                let mask_b = unsafe { _mm256_set1_epi32(0x0000_00FF) };
                let mask_ga = unsafe { _mm256_set1_epi32(0xFF00_FF00u32 as i32) };

                unsafe {
                    row_buffer[..width].copy_from_slice(row_pixels);
                    let src_ptr = row_buffer.as_ptr();
                    let dst_ptr = row_pixels.as_mut_ptr();

                    let mut x = 0;

                    // 1. Left Edge (Scalar)
                    while x < offset && x < width {
                        let p_center = *src_ptr.add(x);
                        let g = (p_center >> 8) & 0xFF;
                        let a = (p_center >> 24) & 0xFF;

                        let r = 0;

                        let b = if x.saturating_add(offset) < width {
                            *src_ptr.add(x + offset) & 0xFF
                        } else {
                            0
                        };

                        *dst_ptr.add(x) = (a << 24) | (r << 16) | (g << 8) | b;
                        x += 1;
                    }

                    // 2. SIMD Loop
                    if offset + 32 <= width {
                        let simd_limit_unrolled = width - offset - 32;
                        while x <= simd_limit_unrolled {
                            let process_block = |off: usize| {
                                let v_center = _mm256_loadu_si256(src_ptr.add(x + off).cast());
                                let v_left =
                                    _mm256_loadu_si256(src_ptr.add(x + off - offset).cast());
                                let v_right =
                                    _mm256_loadu_si256(src_ptr.add(x + off + offset).cast());

                                let ga = _mm256_and_si256(v_center, mask_ga);
                                let r = _mm256_and_si256(v_left, mask_r);
                                let b = _mm256_and_si256(v_right, mask_b);

                                let res = _mm256_or_si256(ga, _mm256_or_si256(r, b));
                                _mm256_storeu_si256(dst_ptr.add(x + off).cast(), res);
                            };

                            process_block(0);
                            process_block(8);
                            process_block(16);
                            process_block(24);

                            x += 32;
                        }
                    }

                    // 3. Tail Loop (Scalar)
                    while x < width {
                        let p_center = *src_ptr.add(x);
                        let g = (p_center >> 8) & 0xFF;
                        let a = (p_center >> 24) & 0xFF;

                        let r = if x >= offset {
                            (*src_ptr.add(x - offset) >> 16) & 0xFF
                        } else {
                            0
                        };

                        let b = if x + offset < width {
                            *src_ptr.add(x + offset) & 0xFF
                        } else {
                            0
                        };

                        *dst_ptr.add(x) = (a << 24) | (r << 16) | (g << 8) | b;
                        x += 1;
                    }
                }
            });
        });
    }"""

if old_avx in content:
    content = content.replace(old_avx, new_avx)
    print("Replaced successfully")
else:
    print("Could not find old_avx. Look at the mismatch:")

with open("crates/abrash-render/src/post_process/filters.rs", "w") as f:
    f.write(content)
