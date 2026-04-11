#!/bin/bash
for file in $(find benches examples tests -name "*.rs"); do
    if grep -q "#!\[allow(missing_docs)\]" "$file"; then
        continue
    fi
    sed -i '1i #![allow(missing_docs)]' "$file"
done
