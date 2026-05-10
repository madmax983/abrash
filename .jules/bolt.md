## L-System string expansion
**Learning:** In exponential string generation algorithms (like L-Systems), replacing an initial `.clone()` on the base string with `String::with_capacity(max_capacity)` followed by `.push_str()` eliminates repetitive dynamic reallocation overhead as the string grows.
**Action:** Always pre-allocate strings to their maximum or estimated needed bounds instead of using `.clone()` if they are expected to grow inside a loop, especially in recursive or exponential generation contexts.
