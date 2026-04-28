import re

with open("crates/abrash-render/src/rasterizer/tile.rs", "r") as f:
    content = f.read()

# Add resize method to AlignedBuffer
resize_method = """    fn resize(&mut self, new_len: usize, default_value: T) {
        if new_len > self.len {
            let align_bytes = 32;
            let elem_size = std::mem::size_of::<T>();
            let extra_elements = (align_bytes + elem_size - 1) / elem_size;

            // Only reallocate if underlying vector capacity isn't enough
            if new_len + extra_elements > self._data.capacity() {
                self._data.resize(new_len + extra_elements, default_value);

                let start_ptr = self._data.as_mut_ptr();
                let start_addr = start_ptr as usize;
                let offset_bytes = (align_bytes - (start_addr % align_bytes)) % align_bytes;
                let offset_elements = offset_bytes / elem_size;
                self.ptr = unsafe { start_ptr.add(offset_elements) };
            }
        }
        self.len = new_len;
    }
}"""

content = content.replace("    }\n}\n\nimpl<T> Deref for AlignedBuffer<T> {", "    }\n\n" + resize_method + "\n\nimpl<T> Deref for AlignedBuffer<T> {")

with open("crates/abrash-render/src/rasterizer/tile.rs", "w") as f:
    f.write(content)
