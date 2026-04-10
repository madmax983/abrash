import re

with open("crates/abrash-anim/src/timeline.rs", "r") as f:
    content = f.read()

# Delete SequenceBuilder, TweenSegmentBuilder and Timeline::sequence
content = re.sub(r'    /// Start building a multi-segment timeline via chained calls\.\n    pub fn sequence\(\) -> SequenceBuilder<T> \{\n        SequenceBuilder \{\n            segments: Vec::new\(\),\n        \}\n    \}\n', '', content)

content = re.sub(r'// --- Builders ---\n.*', '', content, flags=re.DOTALL)

with open("crates/abrash-anim/src/timeline.rs", "w") as f:
    f.write(content)
