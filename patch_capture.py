import re

with open("crates/abrash-gpu-render/src/capture.rs", "r") as f:
    content = f.read()

# Add a test that triggers the panic in CaptureConfig::new
new_test = """
    #[test]
    #[should_panic(expected = "capture width overflowed RGBA byte count")]
    fn test_capture_target_dimensions_overflow() {
        CaptureConfig::new(u32::MAX, 240);
    }
}"""

content = content.replace("    }\n}", "    }" + new_test)

with open("crates/abrash-gpu-render/src/capture.rs", "w") as f:
    f.write(content)
