import re

filepath = 'crates/abrash-render/src/render_api/draw_list.rs'
with open(filepath, 'r') as f:
    content = f.read()

search = r"""    pub fn with_capacity\(
        camera: FrameCamera,
        num_commands: usize,
        num_vertices: usize,
        num_lights: usize,
    \) -> Self \{
        Self \{
            camera,
            lights: Vec::with_capacity\(num_lights\),
            vertices: Vec::with_capacity\(num_vertices\),
            batches: Vec::with_capacity\(num_commands\),
            clear_color: Some\(0xFF00_0000\),
        \}
    \}"""

replace = """    pub fn with_capacity(
        camera: FrameCamera,
        num_commands: usize,
        num_vertices: usize,
        num_lights: usize,
    ) -> Self {
        // WARDEN DEFENSE: Prevent capacity overflow panics
        if num_lights > (isize::MAX as usize) / std::mem::size_of::<Light>() {
            panic!("capacity overflow");
        }
        if num_vertices > (isize::MAX as usize) / std::mem::size_of::<(Vec3, f32)>() {
            panic!("capacity overflow");
        }
        if num_commands > (isize::MAX as usize) / std::mem::size_of::<DrawBatch>() {
            panic!("capacity overflow");
        }

        Self {
            camera,
            lights: Vec::with_capacity(num_lights),
            vertices: Vec::with_capacity(num_vertices),
            batches: Vec::with_capacity(num_commands),
            clear_color: Some(0xFF00_0000),
        }
    }"""

content = re.sub(search, replace, content, count=1)

with open(filepath, 'w') as f:
    f.write(content)
