import re

with open("crates/abrash-render/src/render_api/cpu_renderer.rs", "r") as f:
    content = f.read()

def replacer(match):
    return """
    fn execute_draw_list_owned(&mut self, draw_list: &DrawList, target: &mut RenderTarget) {
        let width = target.width();
        let height = target.height();
        let (pixels, depths) = target.split_mut();
        self.execute_draw_list_inner(draw_list, width, height, pixels, depths);
    }

    /// Execute a pre-built [`DrawList`] into caller-owned buffers.
    pub fn execute_draw_list_into(
        &mut self,
        draw_list: &DrawList,
        target: &mut BorrowedRenderTarget<'_>,
    ) {
        let width = target.width();
        let height = target.height();
        let (pixels, depths) = target.split_mut();
        self.execute_draw_list_inner(draw_list, width, height, pixels, depths);
    }

    fn execute_draw_list_inner(
        &mut self,
        draw_list: &DrawList,
        width: u32,
        height: u32,
        pixels: &mut [u32],
        depths: &mut [f32],
    ) {
        if let Some(color) = draw_list.clear_color {
            pixels.fill(color);
            depths.fill(f32::INFINITY);
        }

        self.tile_renderer.set_clear_color(None); // Disable integrated clearing since we just did a full clear

        self.tile_renderer.begin_frame();
        for batch in &draw_list.batches {
            self.tile_renderer.submit_mesh(
                &batch.indices,
                &draw_list.vertices[batch.vertex_range.start..batch.vertex_range.end],
                batch.color,
            );
        }
        self.tile_renderer
            .end_frame_into_slices(width, height, pixels, depths);
    }
"""

content = re.sub(
    r"\n    fn execute_draw_list_owned\(&mut self, draw_list: &DrawList, target: &mut RenderTarget\) \{.*?\n            \.end_frame_into_slices\(width, height, pixels, depths\);\n    \}",
    replacer,
    content,
    flags=re.DOTALL
)

with open("crates/abrash-render/src/render_api/cpu_renderer.rs", "w") as f:
    f.write(content)
