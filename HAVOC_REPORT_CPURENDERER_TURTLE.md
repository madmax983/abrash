# 👺 Havoc: Thread-Local State Bleed & Turtle OOM

## Vulnerability 1: CpuRenderer Thread-Local State Bleed (Re-entrancy/Concurrency)

* 🧨 **The Trigger:** Multiple `CpuRenderer` instances on the same thread sharing the same `CPU_RENDERER_DRAW_LIST` thread-local variable, leading to re-entrancy panics or cross-contamination of state when rendering.
* 📉 **The Stack Trace:**
  ```
  thread 'test_havoc_cpu_renderer_thread_local' panicked at library/core/src/cell.rs:1049:33:
  already borrowed: BorrowMutError
  ```
* 🧪 **Reproduction:** Run `cargo test --workspace --features nova,parallel --test havoc_cpurenderer_thread_local`.
* 😈 **Comment:** "You assumed `thread_local!` made your renderer safe. You forgot that `RefCell::borrow_mut()` on a shared global state panics on re-entrancy, and multiple renderers on the same thread will silently clobber each other's state. You were wrong."

## Vulnerability 2: LSystem Turtle Out-Of-Memory (OOM)

* 🧨 **The Trigger:** Passing a very large string to `Turtle::generate_mesh(&commands)` which invokes `Mesh::with_capacity()` based on the number of `F` characters directly, bypassing sane memory limits and attempting to allocate hundreds of gigabytes of RAM.
* 📉 **The Stack Trace:**
  ```
  memory allocation of 96000000000 bytes failed
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
  fatal runtime error: Rust panics must be aborted when this type of panic is not configured to be caught.
  ```
* 🧪 **Reproduction:** Run `cargo test --workspace --features nova,parallel --test havoc_turtle_mesh_oom -- --ignored`.
* 😈 **Comment:** "You assumed `with_capacity` was a harmless performance optimization. You forgot that user input is garbage, and allocating 96GB of RAM for an L-System will instantly kill your process. I win."
