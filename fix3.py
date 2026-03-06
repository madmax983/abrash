import re

with open("src/experimental/kuwahara.rs", "r") as f:
    code = f.read()

code = code.replace(
    """    // Copy the filtered pixels back to the framebuffer
    fb.as_mut_slice().copy_from_slice(&new_pixels);
}""",
    """        }
    });
}"""
)

code = code.replace(
    "let mut new_pixels = vec![0u32; (width * height) as usize];",
    ""
)

with open("src/experimental/kuwahara.rs", "w") as f:
    f.write(code)
