import re

with open("src/experimental/kuwahara.rs", "r") as f:
    code = f.read()

code = code.replace(
    """                            for py in py_start..=py_end {
                                let row_offset = (py * width) as usize;
                                for px in px_start..=px_end {
                                    let pixel =
                                        unsafe { *src_fb.get_unchecked(row_offset + px as usize) };""",
    """                            for py in py_start..=py_end {
                                let row_offset = (py * width) as usize;
                                for px in px_start..=px_end {
                                    let pixel = unsafe { *src_fb.get_unchecked(row_offset + px as usize) };"""
)

with open("src/experimental/kuwahara.rs", "w") as f:
    f.write(code)
