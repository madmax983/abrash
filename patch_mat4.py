import re

with open('crates/abrash-core/src/math/mat4.rs', 'r') as f:
    content = f.read()

search = r'''        for \(p, out\) in points\.iter\(\)\.zip\(output\.iter_mut\(\)\) \{
            let x = p\.x \* m00 \+ p\.y \* m10 \+ p\.z \* m20 \+ m30;
            let y = p\.x \* m01 \+ p\.y \* m11 \+ p\.z \* m21 \+ m31;
            let z = p\.x \* m02 \+ p\.y \* m12 \+ p\.z \* m22 \+ m32;
            let w = p\.x \* m03 \+ p\.y \* m13 \+ p\.z \* m23 \+ m33;
            out\.write\(\(Vec3::new\(x, y, z\), w\)\);
        \}'''

replace = r'''        for (p, out) in points.iter().zip(output.iter_mut()) {
            // ⚡ Bolt: Explicitly extract struct fields to local variables to improve instruction
            // scheduling and allow LLVM to better unroll the loop, avoiding repeated memory lookups.
            let px = p.x;
            let py = p.y;
            let pz = p.z;
            let x = px * m00 + py * m10 + pz * m20 + m30;
            let y = px * m01 + py * m11 + pz * m21 + m31;
            let z = px * m02 + py * m12 + pz * m22 + m32;
            let w = px * m03 + py * m13 + pz * m23 + m33;
            out.write((Vec3::new(x, y, z), w));
        }'''

content = re.sub(search, replace, content)

with open('crates/abrash-core/src/math/mat4.rs', 'w') as f:
    f.write(content)
