import re

with open('crates/abrash-render/src/experimental/conway.rs', 'r') as f:
    content = f.read()

replacement = """                            // ⚡ Bolt: Eliminate slow integer modulo in the hot simulation inner loop.
                            // Since dr and dc are strictly in [-1, 1], we can wrap manually using simple conditions.
                            let mut nr = r as isize + dr;
                            if nr < 0 {
                                nr += rows as isize;
                            } else if nr >= rows as isize {
                                nr -= rows as isize;
                            }

                            let mut nc = c as isize + dc;
                            if nc < 0 {
                                nc += cols as isize;
                            } else if nc >= cols as isize {
                                nc -= cols as isize;
                            }

                            let nr = nr as usize;
                            let nc = nc as usize;"""

search = r"""                            // Wrap around boundaries
                            let nr = \(r as isize \+ dr\)\.rem_euclid\(rows as isize\) as usize;
                            let nc = \(c as isize \+ dc\)\.rem_euclid\(cols as isize\) as usize;"""

content = re.sub(search, replacement, content)

with open('crates/abrash-render/src/experimental/conway.rs', 'w') as f:
    f.write(content)
