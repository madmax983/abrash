#!/bin/bash
git add .
git commit -m "🎻 Bard: [documentation update]

📖 Chapter: The \`rasterizer\` module
🔦 Insight: Added missing executable doc-tests and explanations for \`fill_triangle_gouraud\`, \`fill_quad_textured\`, \`fill_quad_textured_gouraud\`, \`fill_triangle_phong\` and \`fill_triangle_phong_shadowed\`. Made the safe assumption to avoid modifying \`const fn\` traits to fix pre-existing Clippy warnings as it was causing compiler cascades.
🧪 Example: Added executable doctests for all the aforementioned APIs that compile properly.
"
