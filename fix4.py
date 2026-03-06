with open("src/experimental/kuwahara.rs", "r") as f:
    code = f.read()

code = code.replace("    #[cfg(feature = \"parallel\")]\n    {\n        new_pixels\n            .par_chunks_mut(width as usize)",
                    "    KUWAHARA_BUFFER.with(|buf| {\n        let mut src_fb_vec = buf.borrow_mut();\n        let size = (width * height) as usize;\n        if src_fb_vec.len() < size {\n            src_fb_vec.resize(size, 0);\n        }\n\n        let src_fb = &mut src_fb_vec[..size];\n        src_fb.copy_from_slice(fb.as_slice());\n\n        let dest_pixels = fb.as_mut_slice();\n\n        #[cfg(feature = \"parallel\")]\n        {\n            dest_pixels\n                .par_chunks_mut(width as usize)")
code = code.replace("    #[cfg(not(feature = \"parallel\"))]\n    {\n        new_pixels\n            .chunks_mut(width as usize)",
                    "    #[cfg(not(feature = \"parallel\"))]\n        {\n            dest_pixels\n                .chunks_mut(width as usize)")
with open("src/experimental/kuwahara.rs", "w") as f:
    f.write(code)
