import re

filepath = 'crates/abrash-render/src/rasterizer/tile.rs'
with open(filepath, 'r') as f:
    content = f.read()

search = r"""    pub fn new\(width: u32, height: u32\) -> Self \{
        assert!\(width > 0 && height > 0, "Dimensions must be positive"\);
        // WARDEN DEFENSE: Prevent integer overflow on expected_len before allocating arrays
        let _ = \(width as usize\)
            \.checked_mul\(height as usize\)
            \.expect\("TileRenderer dimensions overflow"\);"""

replace = """    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0, "Dimensions must be positive");
        // WARDEN DEFENSE: Prevent capacity overflow panics
        let _ = u64::from(width)
            .checked_mul(u64::from(height))
            .filter(|&s| u32::try_from(s).is_ok())
            .expect("capacity overflow");"""

content = re.sub(search, replace, content, count=1)

with open(filepath, 'w') as f:
    f.write(content)
