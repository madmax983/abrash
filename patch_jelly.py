import sys
content = open('crates/abrash-render/src/experimental/jelly.rs', 'r').read()

search = """use std::collections::HashSet;"""
replace = """use foldhash::{HashSet, HashSetExt};"""
content = content.replace(search, replace)

search = """let mut edges = HashSet::with_capacity(max_edges);"""
replace = """let mut edges: HashSet<(usize, usize)> = HashSet::with_capacity(max_edges);"""
content = content.replace(search, replace)

with open('crates/abrash-render/src/experimental/jelly.rs', 'w') as f:
    f.write(content)
print("Patched jelly.rs")
