import re

with open("crates/abrash-render/src/rasterizer/tile.rs", "r") as f:
    content = f.read()

# Replace std::thread_local! buffer
target = """                            std::thread_local! {
                                static TILE_BUFFER: std::cell::RefCell<(Vec<u32>, Vec<f32>)> = const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
                            }
                            TILE_BUFFER.with(|buf| {
                                let mut buffers = buf.borrow_mut();
                                let tile_area = (TILE_SIZE * TILE_SIZE) as usize;
                                if buffers.0.len() < tile_area {
                                    buffers.0.resize(tile_area, 0);
                                    buffers.1.resize(tile_area, f32::INFINITY);
                                }
                                let buffers_ref = &mut *buffers;
                                let tile_pixels = &mut buffers_ref.0;
                                let tile_depths = &mut buffers_ref.1;"""

replacement = """                            std::thread_local! {
                                static TILE_BUFFER: std::cell::RefCell<(AlignedBuffer<u32>, AlignedBuffer<f32>)> = std::cell::RefCell::new((AlignedBuffer::new(0), AlignedBuffer::new(0)));
                            }
                            TILE_BUFFER.with(|buf| {
                                let mut buffers = buf.borrow_mut();
                                let tile_area = (TILE_SIZE * TILE_SIZE) as usize;
                                if buffers.0.len() < tile_area {
                                    buffers.0.resize(tile_area, 0);
                                    buffers.1.resize(tile_area, f32::INFINITY);
                                }
                                let buffers_ref = &mut *buffers;
                                let tile_pixels = &mut buffers_ref.0;
                                let tile_depths = &mut buffers_ref.1;"""

content = content.replace(target, replacement)

# We also need to remove `#[cfg(not(feature = "parallel"))]` from AlignedBuffer struct and impls.
# However, AlignedBuffer does not have `#[cfg(not(feature = "parallel"))]`. It has `#[allow(dead_code)]`.
# So it's fine.

with open("crates/abrash-render/src/rasterizer/tile.rs", "w") as f:
    f.write(content)
