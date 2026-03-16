with open("src/experimental/crt.rs", "r") as f:
    content = f.read()

target = """    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    use std::cell::RefCell;

    thread_local! {"""
repl = """    use std::cell::RefCell;

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    thread_local! {"""
content = content.replace(target, repl)

with open("src/experimental/crt.rs", "w") as f:
    f.write(content)
