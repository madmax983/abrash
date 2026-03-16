with open("src/experimental/crt.rs", "r") as f:
    content = f.read()

target = """pub fn apply_crt(fb: &mut Framebuffer, distortion: f32) {
    if distortion <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    use std::cell::RefCell;

    let cx = width as f32 / 2.0;"""
repl = """use std::cell::RefCell;

pub fn apply_crt(fb: &mut Framebuffer, distortion: f32) {
    if distortion <= 0.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 {
        return;
    }

    let cx = width as f32 / 2.0;"""
content = content.replace(target, repl)
with open("src/experimental/crt.rs", "w") as f:
    f.write(content)
