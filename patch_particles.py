with open("crates/abrash-render/src/particles.rs", "r") as f:
    code = f.read()

import re

new_code = re.sub(
    r"// Update existing particles\n\s*let mut i = 0;\n\s*while i < self\.particles\.len\(\) \{\n\s*let p = &mut self\.particles\[i\];\n\n\s*p\.life -= dt;\n\s*if p\.life <= 0\.0 \{\n\s*// Remove dead particle \(swap remove is O\(1\)\)\n\s*self\.particles\.swap_remove\(i\);\n\s*// Don't increment i, as the swapped element needs to be checked\n\s*continue;\n\s*\}\n\n\s*// Physics\n\s*p\.velocity = p\.velocity \+ self\.gravity \* dt;\n\s*p\.position = p\.position \+ p\.velocity \* dt;\n\n\s*i \+= 1;\n\s*\}",
    """// Update existing particles
        // Unconditional math pass
        let grav = self.gravity * dt;
        for p in &mut self.particles {
            p.life -= dt;
            p.velocity.x += grav.x;
            p.velocity.y += grav.y;
            p.velocity.z += grav.z;
            p.position.x += p.velocity.x * dt;
            p.position.y += p.velocity.y * dt;
            p.position.z += p.velocity.z * dt;
        }

        // Conditional mutation pass
        let mut i = 0;
        while i < self.particles.len() {
            if self.particles[i].life <= 0.0 {
                self.particles.swap_remove(i);
                continue;
            }
            i += 1;
        }""",
    code
)

with open("crates/abrash-render/src/particles.rs", "w") as f:
    f.write(new_code)
