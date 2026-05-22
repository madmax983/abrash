# 👺 Havoc: Undefined Behavior in `PreparedTrianglesList::into_par_iter`

## 🧊 The Trigger
When iterating over `PreparedTrianglesList` in parallel using `into_par_iter()`, an uninitialized read occurs.

## 📉 The Stack Trace
```
error: Undefined Behavior: trying to retag from <143839> for SharedReadWrite permission at alloc46598[0x8], but that tag does not exist in the borrow stack for this location
   --> .../crossbeam-epoch/src/internal.rs:549:9
```
This UB is triggered by the code in `into_par_iter` creating a `[None; 8]` array and mapping memory over padding.

## 🧪 Reproduction
Run the following test using Miri:
```bash
cargo +nightly miri test -p abrash-render --features nova,parallel test_tile_uninit_read_ub
```

## 😈 Comment
You tried to optimize `into_par_iter` using a stack array `[None; 8]` and `assume_init()`. You assumed padding bytes didn't matter. Miri knows better. Your "elided heap allocation" just earned you Undefined Behavior.
