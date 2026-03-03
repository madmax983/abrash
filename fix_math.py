content = open("src/math.rs").read()
if "pub fn length(&self) -> f32" not in content[:1000]:
    content = content.replace("    pub const fn new(x: f32, y: f32) -> Self {\n        Self { x, y }\n    }", "    pub const fn new(x: f32, y: f32) -> Self {\n        Self { x, y }\n    }\n\n    /// Returns the length (magnitude) of the vector.\n    #[must_use]\n    #[inline]\n    pub fn length(&self) -> f32 {\n        self.x.hypot(self.y)\n    }")
    open("src/math.rs", "w").write(content)
