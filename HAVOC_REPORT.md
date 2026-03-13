# 👺 Havoc: Fragility Report

## The Trigger: LSystem Memory Exhaustion (DOS)
* 🧨 **The Trigger:** LSystem `expand()` function relies entirely on exponential memory allocation without bounds checking or safety limits for the generated string size. An input axiom or ruleset with a high iteration count (e.g. 50+) or a highly multiplicative rule causes the application to crash the host operating system's allocator via an Out-Of-Memory (OOM) abort signal.
* 📉 **The Stack Trace:**
```
memory allocation of 10000000000 bytes failed
error: test failed, to rerun pass `--test havoc_exploit`
Caused by:
  process didn't exit successfully: `/app/target/debug/deps/havoc_exploit-3ca42354a5f1887e` (signal: 6, SIGABRT: process abort signal)
```
* 🧪 **Reproduction:** Run `cargo test --test havoc_exploit test_arboretum_expand_dos -- --ignored`
* 😈 **Comment:** "You assumed the user wouldn't want to simulate Yggdrasil. You assumed wrong. Your procedural generation just generated an OOM killer."
