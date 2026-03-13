import re

with open("src/experimental/radial_blur.rs", "r") as f:
    content = f.read()

# Replace inner loop for parallel
content = re.sub(
    r"for \(x, pixel\) in row\.iter_mut\(\)\.enumerate\(\) \{\s+let dx = x as f32 - cx as f32;\s+let dy = y as f32 - cy as f32;\s+let step_x = \(dx \* step_factor \* 65536\.0\) as i32;\s+let step_y = \(dy \* step_factor \* 65536\.0\) as i32;",
    r"""let dy = y as f32 - cy as f32;
                    let step_y = (dy * step_factor * 65536.0) as i32;
                    for (x, pixel) in row.iter_mut().enumerate() {
                        let dx = x as f32 - cx as f32;
                        let step_x = (dx * step_factor * 65536.0) as i32;""",
    content
)

# Replace cur_x and cur_y
content = re.sub(
    r"let mut cur_x = \(x as i32\) << 16;\s+let mut cur_y = \(y as i32\) << 16;",
    r"""let mut cur_x = ((x as i32) << 16) + 32768;
                        let mut cur_y = ((y as i32) << 16) + 32768;""",
    content
)

# Replace inner loop for scalar
content = re.sub(
    r"for x in 0\.\.width \{\s+let dx = x as f32 - cx as f32;\s+let dy = y as f32 - cy as f32;\s+let step_x = \(dx \* step_factor \* 65536\.0\) as i32;\s+let step_y = \(dy \* step_factor \* 65536\.0\) as i32;",
    r"""let dy = y as f32 - cy as f32;
                let step_y = (dy * step_factor * 65536.0) as i32;
                for x in 0..width {
                    let dx = x as f32 - cx as f32;
                    let step_x = (dx * step_factor * 65536.0) as i32;""",
    content
)

with open("src/experimental/radial_blur.rs", "w") as f:
    f.write(content)

