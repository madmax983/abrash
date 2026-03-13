with open("tests/platform_tests.rs", "r") as f:
    text = f.read()

import re
text = re.sub(r'Ok\(Self \{\n\s*event_queue: Vec::new\(\),\n\s*expected_strings: Vec::new\(\),\n\s*written_strings: Vec::new\(\),\n\s*flush_count: 0,\n\s*\}', r'Self {\n            event_queue: Vec::new(),\n            expected_strings: Vec::new(),\n            written_strings: Vec::new(),\n            flush_count: 0,\n        }', text)
text = text.replace("}\n    }\n\n    fn set_event", "}\n\n    fn set_event")

with open("tests/platform_tests.rs", "w") as f:
    f.write(text)
