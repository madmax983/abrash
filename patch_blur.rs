use std::fs;

fn main() {
    let mut content = fs::read_to_string("src/post_process/blur.rs").unwrap();
    content = content.replace("par_chunks_exact_mut", "par_chunks_mut");
    content = content.replace("chunks_exact_mut", "chunks_mut");
    fs::write("src/post_process/blur.rs", content).unwrap();
}
