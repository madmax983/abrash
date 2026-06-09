sed -i 's/pub fn new(window: Arc<Window>)/pub fn new(ctx: WindowContext<'_>)/g' src/platform/winit.rs
sed -i 's/SoftbufferContext::new(window.clone())/SoftbufferContext::new(ctx.window.clone())/g' src/platform/winit.rs
sed -i 's/SoftbufferSurface::new(&context, window)/SoftbufferSurface::new(&context, ctx.window.clone())/g' src/platform/winit.rs
