import re

filepath = 'crates/abrash-core/src/hiz_buffer.rs'
with open(filepath, 'r') as f:
    content = f.read()

search1 = r"""    fn new\(width: u32, height: u32\) -> Self \{
        let _ = \(width as usize\)
            \.checked_mul\(height as usize\)
            \.and_then\(\|a\| a\.checked_mul\(4\)\)
            \.expect\("Hi-Z dimensions overflow"\);
        let size = \(width as usize\) \* \(height as usize\);"""

replace1 = """    fn new(width: u32, height: u32) -> Self {
        // Prevent capacity overflow when allocating vec![f32::INFINITY; size]
        let size = u64::from(width)
            .checked_mul(u64::from(height))
            .filter(|&s| u32::try_from(s).is_ok())
            .expect("Hi-Z dimensions overflow") as usize;"""

content = re.sub(search1, replace1, content, count=1)

search2 = r"""    pub fn new\(width: u32, height: u32\) -> Self \{
        assert!\(width > 0 && height > 0, "Dimensions must be positive"\);

        // Calculate level count: ceil\(log2\(max\(width, height\)\)\) \+ 1
        // This gives us levels 0..level_count where level 0 is full resolution
        let max_dim = width\.max\(height\) as f32;"""

replace2 = """    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0, "Dimensions must be positive");

        // Prevent huge allocations up front
        let _ = u64::from(width)
            .checked_mul(u64::from(height))
            .filter(|&s| u32::try_from(s).is_ok())
            .expect("capacity overflow");

        // Calculate level count: ceil(log2(max(width, height))) + 1
        // This gives us levels 0..level_count where level 0 is full resolution
        let max_dim = width.max(height) as f32;"""

content = re.sub(search2, replace2, content, count=1)

with open(filepath, 'w') as f:
    f.write(content)
