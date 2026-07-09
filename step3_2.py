with open('crates/abrash-render/src/post_process/mod.rs', 'r') as f:
    content = f.read()

content = content.replace('pub use self::filters::*;', 'pub use self::filters::*;\npub use filters::apply_exposure;')

with open('crates/abrash-render/src/post_process/mod.rs', 'w') as f:
    f.write(content)
