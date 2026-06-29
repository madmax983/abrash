import re

with open('crates/abrash-core/src/transform.rs', 'r') as f:
    content = f.read()

def replacer(match):
    prefix = match.group(1)
    body = match.group(2)
    suffix = match.group(3)

    # We want to replace `out.extend(points.iter().map(|&p| { ... }));`
    # with `out.extend(points.iter().map(|p| { ... }));`

    body = body.replace('points.iter().map(|&p|', 'points.iter().map(|p|')
    # fix the body mapping to use `p.x` instead of `(*p).x` if we had `&p`, but since `p` is a reference, `p.x` works because of auto-deref in Rust!
    # Let's actually just change the function to use `out.resize` and `clone_from_slice` if possible, or just preallocate and assign!

    return prefix + body + suffix

# Actually, the simplest performance win without lifetime issues here is removing the iter().map().collect() or iter().map() into extend(),
# but as we know from earlier `extend` already has some overhead, let's just make it assign directly to initialized memory.

# However, since `out.extend(iter.map())` uses `TrustedLen` in stdlib for slice iterators, it's actually highly optimized and lowers to memmove or vectorized operations in release mode.
