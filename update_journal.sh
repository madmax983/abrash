# Remove the old lesson
sed -i '/\*\*Lesson:\*\* Using a persistent `thread_local!` accumulator buffer is an effective/d' .jules/nova.md

# Add the new lesson
cat << 'INNER_EOF' >> .jules/nova.md
**Lesson:** While using `thread_local!` might seem like a quick way to maintain post-processing state across frames, it is an architectural anti-pattern that hides global state, causes test flakiness across parallel test runners, and prevents robust reusability. Encapsulating the temporal buffer inside a dedicated, stateful `struct` (e.g., `MotionBlur`) ensures clean execution and predictable behavior.
INNER_EOF
