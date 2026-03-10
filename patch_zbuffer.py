import re

with open("src/zbuffer.rs", "r") as f:
    content = f.read()

# Let's check `get_depth` as well in ZBuffer
old_func = """    pub fn get_depth(&self, x: i32, y: i32) -> Option<f32> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let idx = (y as u32 * self.width + x as u32) as usize;
        Some(self.depths[idx])
    }"""

new_func = """    pub fn get_depth(&self, x: i32, y: i32) -> Option<f32> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let idx = (y as u32 * self.width + x as u32) as usize;
        // SAFETY: bounds checked above.
        Some(unsafe { *self.depths.get_unchecked(idx) })
    }"""

content = content.replace(old_func, new_func)

old_func2 = """    pub fn test_and_set(&mut self, x: i32, y: i32, depth: f32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }

        let idx = (y as u32 * self.width + x as u32) as usize;
        if depth < self.depths[idx] {
            self.depths[idx] = depth;
            true
        } else {
            false
        }
    }"""

new_func2 = """    pub fn test_and_set(&mut self, x: i32, y: i32, depth: f32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return false;
        }

        let idx = (y as u32 * self.width + x as u32) as usize;
        // SAFETY: The bounds are strictly checked by the if statement above.
        unsafe {
            let d = self.depths.get_unchecked_mut(idx);
            if depth < *d {
                *d = depth;
                true
            } else {
                false
            }
        }
    }"""

content = content.replace(old_func2, new_func2)

with open("src/zbuffer.rs", "w") as f:
    f.write(content)
