1. **Refactor `PoolEntry<T>` in `crates/abrash-render/src/render_api/handles.rs`**
   - The custom `enum PoolEntry<T> { Occupied { value: T, generation: u32 }, Vacant { generation: u32 } }` is unnecessarily complex and non-idiomatic.
   - Replace it with a simpler struct: `struct Slot<T> { value: Option<T>, generation: u32 }`.
   - Update `ResourcePool` methods (`new`, `with_capacity`, `insert`, `get`, `get_mut`, `remove`) to use standard `Option` semantics (`is_some`, `is_none`, `take()`).
   - This eliminates the custom enum, flattens the match statements, and simplifies memory reclamation.
2. **Refactor `DropState` in `crates/abrash-render/src/experimental/precipitation.rs`**
   - The `enum DropState { Falling, Splashing }` is just a boolean in disguise.
   - Replace it with `is_splashing: bool` in the `Drop` struct.
   - Update `apply_precipitation` and tests to use `if drop.is_splashing` and `if !drop.is_splashing`.
   - This removes an unnecessary enum type.
3. **Run Pre-Commit Checks**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
4. **Submit PR**
   - Create a PR with the title '🪒 Razor: Flatten PoolEntry and DropState enums' detailing the cuts made.
