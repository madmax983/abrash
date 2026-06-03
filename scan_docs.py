import os
import re

def scan_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # Check for missing module-level docs
    has_mod_doc = re.search(r'^\s*//!', content, re.MULTILINE)

    # Find all public structs, enums, functions
    pub_items = re.findall(r'^(?!.*//)(?:pub\s+)?(?:unsafe\s+)?(?:fn|struct|enum|trait)\s+(\w+)', content, re.MULTILINE)

    return has_mod_doc is not None, pub_items

def main():
    for root, _, files in os.walk('.'):
        if '.git' in root or 'target' in root or 'benches' in root or 'tests' in root or 'examples' in root:
            continue
        for file in files:
            if file.endswith('.rs'):
                filepath = os.path.join(root, file)
                has_mod, items = scan_file(filepath)
                if not has_mod or len(items) > 0:
                    # simplistic check, just to find places with no docs
                    with open(filepath, 'r', encoding='utf-8') as f:
                        lines = f.readlines()
                    missing_docs_items = []
                    for i, line in enumerate(lines):
                        if re.match(r'^\s*pub\s+(?:unsafe\s+)?(?:fn|struct|enum|trait)\s+', line):
                            if i == 0 or not re.match(r'^\s*///', lines[i-1]):
                                # ignore #[derive...] and such
                                if i == 0 or not (lines[i-1].strip().startswith('#[') or lines[i-1].strip().startswith('///')):
                                    missing_docs_items.append(line.strip())

                    if not has_mod or missing_docs_items:
                        print(f"File: {filepath}")
                        if not has_mod:
                            print(f"  Missing module docs")
                        for item in missing_docs_items:
                            print(f"  Missing item docs: {item}")

if __name__ == "__main__":
    main()
